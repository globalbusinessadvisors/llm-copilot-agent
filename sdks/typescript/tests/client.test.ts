import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { HttpClient, CopilotError } from '../src/client';
import type { CopilotConfig } from '../src/types';

// Mock fetch globally
const mockFetch = vi.fn();
global.fetch = mockFetch;

describe('HttpClient', () => {
  const config: CopilotConfig = {
    baseUrl: 'https://api.example.com',
    apiKey: 'test-api-key',
    tenantId: 'test-tenant',
    timeout: 5000,
    maxRetries: 2,
  };

  let client: HttpClient;

  beforeEach(() => {
    client = new HttpClient(config);
    mockFetch.mockReset();
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  describe('GET requests', () => {
    it('should make a successful GET request', async () => {
      const responseData = { id: '123', name: 'Test' };

      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: () => Promise.resolve(responseData),
        headers: new Headers({
          'X-Request-ID': 'req-123',
          'X-Processing-Time': '50',
        }),
      });

      const result = await client.get<typeof responseData>('/api/test');

      expect(result.success).toBe(true);
      expect(result.data).toEqual(responseData);
      expect(result.metadata?.requestId).toBe('req-123');

      expect(mockFetch).toHaveBeenCalledWith(
        'https://api.example.com/api/test',
        expect.objectContaining({
          method: 'GET',
          headers: expect.any(Headers),
        })
      );
    });

    it('should include query parameters', async () => {
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: () => Promise.resolve({}),
        headers: new Headers(),
      });

      await client.get('/api/test', { page: 1, limit: 10, active: true });

      expect(mockFetch).toHaveBeenCalledWith(
        'https://api.example.com/api/test?page=1&limit=10&active=true',
        expect.any(Object)
      );
    });

    it('should include authorization header', async () => {
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: () => Promise.resolve({}),
        headers: new Headers(),
      });

      await client.get('/api/test');

      const [, options] = mockFetch.mock.calls[0] as [string, RequestInit];
      const headers = options.headers as Headers;

      expect(headers.get('Authorization')).toBe('Bearer test-api-key');
    });

    it('should include tenant header when configured', async () => {
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: () => Promise.resolve({}),
        headers: new Headers(),
      });

      await client.get('/api/test');

      const [, options] = mockFetch.mock.calls[0] as [string, RequestInit];
      const headers = options.headers as Headers;

      expect(headers.get('X-Tenant-ID')).toBe('test-tenant');
    });
  });

  describe('POST requests', () => {
    it('should make a successful POST request with body', async () => {
      const requestBody = { name: 'Test', value: 42 };
      const responseData = { id: '123', ...requestBody };

      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: () => Promise.resolve(responseData),
        headers: new Headers(),
      });

      const result = await client.post<typeof responseData>('/api/test', requestBody);

      expect(result.success).toBe(true);
      expect(result.data).toEqual(responseData);

      expect(mockFetch).toHaveBeenCalledWith(
        'https://api.example.com/api/test',
        expect.objectContaining({
          method: 'POST',
          body: JSON.stringify(requestBody),
        })
      );
    });
  });

  describe('Error handling', () => {
    it('should throw CopilotError on API error', async () => {
      mockFetch.mockResolvedValueOnce({
        ok: false,
        status: 400,
        statusText: 'Bad Request',
        json: () =>
          Promise.resolve({
            code: 'VALIDATION_ERROR',
            message: 'Invalid input',
          }),
        headers: new Headers({
          'X-Request-ID': 'req-456',
        }),
      });

      await expect(client.get('/api/test')).rejects.toThrow(CopilotError);

      try {
        await client.get('/api/test');
      } catch (error) {
        // Reset mock for this call
      }
    });

    it('should handle network errors', async () => {
      mockFetch.mockRejectedValueOnce(new Error('Network failure'));
      mockFetch.mockRejectedValueOnce(new Error('Network failure'));
      mockFetch.mockRejectedValueOnce(new Error('Network failure'));

      await expect(client.get('/api/test')).rejects.toThrow('Network failure');
    });

    it('should handle AbortError', async () => {
      // Mock an abort error directly for all retry attempts
      const abortError = Object.assign(new Error('Aborted'), { name: 'AbortError' });
      mockFetch.mockRejectedValue(abortError);

      await expect(client.get('/api/test')).rejects.toThrow('Request timeout');
    });
  });

  describe('Retry logic', () => {
    it('should retry on server errors', async () => {
      mockFetch
        .mockResolvedValueOnce({
          ok: false,
          status: 500,
          statusText: 'Internal Server Error',
          json: () => Promise.resolve({ code: 'SERVER_ERROR' }),
          headers: new Headers(),
        })
        .mockResolvedValueOnce({
          ok: true,
          json: () => Promise.resolve({ success: true }),
          headers: new Headers(),
        });

      const result = await client.get('/api/test');

      expect(result.success).toBe(true);
      expect(mockFetch).toHaveBeenCalledTimes(2);
    });

    it('should not retry on client errors (except 429)', async () => {
      mockFetch.mockResolvedValueOnce({
        ok: false,
        status: 400,
        statusText: 'Bad Request',
        json: () => Promise.resolve({ code: 'BAD_REQUEST' }),
        headers: new Headers(),
      });

      await expect(client.get('/api/test')).rejects.toThrow();
      expect(mockFetch).toHaveBeenCalledTimes(1);
    });

    it('should retry on rate limit (429)', async () => {
      mockFetch
        .mockResolvedValueOnce({
          ok: false,
          status: 429,
          statusText: 'Too Many Requests',
          json: () => Promise.resolve({ code: 'RATE_LIMITED' }),
          headers: new Headers(),
        })
        .mockResolvedValueOnce({
          ok: true,
          json: () => Promise.resolve({ success: true }),
          headers: new Headers(),
        });

      const result = await client.get('/api/test');

      expect(result.success).toBe(true);
      expect(mockFetch).toHaveBeenCalledTimes(2);
    });
  });

  describe('Rate limit headers', () => {
    it('should parse rate limit headers', async () => {
      const resetTime = Math.floor(Date.now() / 1000) + 60;

      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: () => Promise.resolve({}),
        headers: new Headers({
          'X-RateLimit-Limit': '100',
          'X-RateLimit-Remaining': '95',
          'X-RateLimit-Reset': String(resetTime),
        }),
      });

      const result = await client.get('/api/test');

      expect(result.metadata?.rateLimit).toBeDefined();
      expect(result.metadata?.rateLimit?.limit).toBe(100);
      expect(result.metadata?.rateLimit?.remaining).toBe(95);
    });
  });
});

describe('CopilotError', () => {
  it('should create error with all properties', () => {
    const error = new CopilotError(
      'Test error',
      'TEST_CODE',
      400,
      { field: 'value' },
      'req-123'
    );

    expect(error.message).toBe('Test error');
    expect(error.code).toBe('TEST_CODE');
    expect(error.status).toBe(400);
    expect(error.details).toEqual({ field: 'value' });
    expect(error.requestId).toBe('req-123');
    expect(error.name).toBe('CopilotError');
  });

  it('should create error from API error', () => {
    const error = CopilotError.fromApiError(
      {
        code: 'API_ERROR',
        message: 'API error message',
        details: { reason: 'test' },
        requestId: 'req-456',
      },
      500
    );

    expect(error.message).toBe('API error message');
    expect(error.code).toBe('API_ERROR');
    expect(error.status).toBe(500);
  });
});
