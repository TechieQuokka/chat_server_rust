/**
 * Chat Server v3 - Load Test Suite
 *
 * This k6 script tests the chat server under various load conditions.
 *
 * Usage:
 *   k6 run tests/k6/load-test.js
 *   k6 run --vus 100 --duration 5m tests/k6/load-test.js
 *   k6 run --out json=results.json tests/k6/load-test.js
 */

import http from 'k6/http';
import ws from 'k6/ws';
import { check, sleep, group } from 'k6';
import { Rate, Trend, Counter } from 'k6/metrics';
import { randomString, randomIntBetween } from 'https://jslib.k6.io/k6-utils/1.4.0/index.js';

// ============================================
// Configuration
// ============================================

const BASE_URL = __ENV.BASE_URL || 'http://localhost:8080';
const WS_URL = __ENV.WS_URL || 'ws://localhost:8080/gateway';
const API_VERSION = '/api/v1';

// ============================================
// Custom Metrics
// ============================================

const errorRate = new Rate('errors');
const loginDuration = new Trend('login_duration', true);
const messageSendDuration = new Trend('message_send_duration', true);
const wsConnectionDuration = new Trend('ws_connection_duration', true);
const wsMessageLatency = new Trend('ws_message_latency', true);
const messagesReceived = new Counter('messages_received');
const wsConnections = new Counter('ws_connections');

// ============================================
// Test Options
// ============================================

export const options = {
  scenarios: {
    // Smoke test - quick sanity check
    smoke: {
      executor: 'constant-vus',
      vus: 5,
      duration: '1m',
      tags: { scenario: 'smoke' },
      exec: 'smokeTest',
    },

    // Load test - sustained traffic
    load: {
      executor: 'ramping-vus',
      startVUs: 0,
      stages: [
        { duration: '2m', target: 50 },   // Ramp up
        { duration: '5m', target: 50 },   // Steady state
        { duration: '2m', target: 100 },  // Peak load
        { duration: '3m', target: 100 },  // Sustained peak
        { duration: '2m', target: 0 },    // Ramp down
      ],
      tags: { scenario: 'load' },
      exec: 'loadTest',
      startTime: '1m', // Start after smoke test
    },

    // Stress test - find breaking point
    stress: {
      executor: 'ramping-vus',
      startVUs: 0,
      stages: [
        { duration: '2m', target: 100 },
        { duration: '3m', target: 200 },
        { duration: '3m', target: 300 },
        { duration: '3m', target: 400 },
        { duration: '2m', target: 0 },
      ],
      tags: { scenario: 'stress' },
      exec: 'stressTest',
      startTime: '15m', // Start after load test
    },

    // WebSocket test - concurrent connections
    websocket: {
      executor: 'constant-vus',
      vus: 50,
      duration: '5m',
      tags: { scenario: 'websocket' },
      exec: 'websocketTest',
      startTime: '1m',
    },
  },

  thresholds: {
    'http_req_duration': ['p(95)<500', 'p(99)<1000'],
    'http_req_failed': ['rate<0.01'],
    'errors': ['rate<0.05'],
    'login_duration': ['p(95)<1000'],
    'message_send_duration': ['p(95)<200'],
    'ws_connection_duration': ['p(95)<2000'],
    'ws_message_latency': ['p(95)<100'],
  },
};

// ============================================
// Helper Functions
// ============================================

function getHeaders(token = null) {
  const headers = {
    'Content-Type': 'application/json',
    'Accept': 'application/json',
  };
  if (token) {
    headers['Authorization'] = `Bearer ${token}`;
  }
  return headers;
}

function login(username, password) {
  const startTime = Date.now();
  const res = http.post(
    `${BASE_URL}${API_VERSION}/auth/login`,
    JSON.stringify({ username, password }),
    { headers: getHeaders() }
  );
  loginDuration.add(Date.now() - startTime);

  const success = check(res, {
    'login successful': (r) => r.status === 200,
    'has access token': (r) => r.json('access_token') !== undefined,
  });

  if (!success) {
    errorRate.add(1);
    return null;
  }

  return res.json('access_token');
}

function register(username, email, password) {
  const res = http.post(
    `${BASE_URL}${API_VERSION}/auth/register`,
    JSON.stringify({ username, email, password }),
    { headers: getHeaders() }
  );

  return check(res, {
    'registration successful': (r) => r.status === 201 || r.status === 409,
  });
}

// ============================================
// Test Scenarios
// ============================================

export function smokeTest() {
  group('Health Check', () => {
    const res = http.get(`${BASE_URL}/health`);
    check(res, {
      'health check passed': (r) => r.status === 200,
    }) || errorRate.add(1);
  });

  group('API Availability', () => {
    const res = http.get(`${BASE_URL}${API_VERSION}/users/@me`, {
      headers: getHeaders(),
    });
    check(res, {
      'API responds': (r) => r.status === 401 || r.status === 200,
    }) || errorRate.add(1);
  });

  sleep(1);
}

export function loadTest() {
  const vuId = __VU;
  const username = `loadtest_user_${vuId}`;
  const email = `loadtest${vuId}@test.local`;
  const password = 'LoadTest123!';

  // Register (idempotent)
  register(username, email, password);

  // Login
  const token = login(username, password);
  if (!token) {
    sleep(1);
    return;
  }

  group('User Operations', () => {
    // Get current user
    const meRes = http.get(`${BASE_URL}${API_VERSION}/users/@me`, {
      headers: getHeaders(token),
    });
    check(meRes, {
      'get user successful': (r) => r.status === 200,
    }) || errorRate.add(1);

    // Get guilds
    const guildsRes = http.get(`${BASE_URL}${API_VERSION}/users/@me/guilds`, {
      headers: getHeaders(token),
    });
    check(guildsRes, {
      'get guilds successful': (r) => r.status === 200,
    }) || errorRate.add(1);
  });

  group('Message Operations', () => {
    const channelId = '2'; // General channel from seed data

    // Get messages
    const messagesRes = http.get(
      `${BASE_URL}${API_VERSION}/channels/${channelId}/messages?limit=50`,
      { headers: getHeaders(token) }
    );
    check(messagesRes, {
      'get messages successful': (r) => r.status === 200 || r.status === 403,
    }) || errorRate.add(1);

    // Send message
    const startTime = Date.now();
    const sendRes = http.post(
      `${BASE_URL}${API_VERSION}/channels/${channelId}/messages`,
      JSON.stringify({
        content: `Load test message from VU ${vuId} at ${new Date().toISOString()}`,
      }),
      { headers: getHeaders(token) }
    );
    messageSendDuration.add(Date.now() - startTime);

    check(sendRes, {
      'send message successful': (r) => r.status === 201 || r.status === 403,
    }) || errorRate.add(1);
  });

  sleep(randomIntBetween(1, 3));
}

export function stressTest() {
  const vuId = __VU;
  const iteration = __ITER;

  // Rapid-fire requests to stress the server
  const requests = [
    ['GET', `${BASE_URL}/health`, null],
    ['GET', `${BASE_URL}${API_VERSION}/guilds/1`, null],
    ['GET', `${BASE_URL}${API_VERSION}/channels/2/messages?limit=10`, null],
  ];

  for (const [method, url, body] of requests) {
    const res = method === 'GET'
      ? http.get(url, { headers: getHeaders() })
      : http.post(url, body, { headers: getHeaders() });

    check(res, {
      'response received': (r) => r.status !== 0,
      'not server error': (r) => r.status < 500,
    }) || errorRate.add(1);
  }

  sleep(0.1); // Minimal sleep for stress test
}

export function websocketTest() {
  const vuId = __VU;
  const username = `wstest_user_${vuId}`;
  const email = `wstest${vuId}@test.local`;
  const password = 'WsTest123!';

  // Register and login
  register(username, email, password);
  const token = login(username, password);
  if (!token) {
    sleep(1);
    return;
  }

  const connectionStart = Date.now();

  const res = ws.connect(`${WS_URL}?v=1&encoding=json`, {}, function(socket) {
    wsConnections.add(1);

    socket.on('open', () => {
      wsConnectionDuration.add(Date.now() - connectionStart);

      // Send IDENTIFY payload
      socket.send(JSON.stringify({
        op: 2, // IDENTIFY
        d: {
          token: token,
          properties: {
            os: 'k6-test',
            browser: 'k6',
            device: 'load-test',
          },
        },
      }));
    });

    socket.on('message', (data) => {
      const message = JSON.parse(data);
      messagesReceived.add(1);

      switch (message.op) {
        case 10: // HELLO
          // Start heartbeat
          const heartbeatInterval = message.d.heartbeat_interval;
          socket.setInterval(() => {
            socket.send(JSON.stringify({
              op: 1, // HEARTBEAT
              d: null,
            }));
          }, heartbeatInterval);
          break;

        case 0: // DISPATCH
          if (message.t === 'READY') {
            console.log(`VU ${vuId}: Connected and ready`);
          }
          break;

        case 11: // HEARTBEAT_ACK
          // Heartbeat acknowledged
          break;
      }
    });

    socket.on('error', (e) => {
      errorRate.add(1);
      console.error(`VU ${vuId}: WebSocket error: ${e.error()}`);
    });

    socket.on('close', () => {
      console.log(`VU ${vuId}: WebSocket closed`);
    });

    // Keep connection open for the duration
    socket.setTimeout(() => {
      // Send a test message through WebSocket
      const sendStart = Date.now();
      socket.send(JSON.stringify({
        op: 0, // DISPATCH
        t: 'MESSAGE_CREATE',
        d: {
          channel_id: '2',
          content: `WebSocket test message from VU ${vuId}`,
        },
      }));
      wsMessageLatency.add(Date.now() - sendStart);

      socket.close();
    }, 60000); // 60 second connection
  });

  check(res, {
    'WebSocket connected': (r) => r && r.status === 101,
  }) || errorRate.add(1);
}

// ============================================
// Default Function (for simple runs)
// ============================================

export default function() {
  loadTest();
}

// ============================================
// Setup and Teardown
// ============================================

export function setup() {
  console.log('Starting load test suite...');
  console.log(`Target: ${BASE_URL}`);

  // Verify server is up
  const res = http.get(`${BASE_URL}/health`);
  if (res.status !== 200) {
    throw new Error(`Server not ready: ${res.status}`);
  }

  return { startTime: Date.now() };
}

export function teardown(data) {
  const duration = (Date.now() - data.startTime) / 1000;
  console.log(`Load test completed in ${duration.toFixed(2)} seconds`);
}
