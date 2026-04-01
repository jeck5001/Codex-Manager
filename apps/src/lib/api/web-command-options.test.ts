import test from 'node:test';
import assert from 'node:assert/strict';

import { resolveWebCommandRequestOptions } from './web-command-options.ts';

test('account auth recovery uses extended timeout without retries in web runtime', () => {
  assert.deepEqual(resolveWebCommandRequestOptions('service_account_auth_recover'), {
    timeoutMs: 120000,
    retries: 0,
  });
});

test('explicit request options can still override command defaults', () => {
  assert.deepEqual(
    resolveWebCommandRequestOptions('service_account_auth_recover', {
      timeoutMs: 5000,
      retries: 2,
    }),
    {
      timeoutMs: 5000,
      retries: 2,
    }
  );
});

test('unconfigured commands keep caller options unchanged', () => {
  assert.deepEqual(
    resolveWebCommandRequestOptions('service_usage_refresh', {
      timeoutMs: 8000,
    }),
    {
      timeoutMs: 8000,
    }
  );
});
