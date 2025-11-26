/**
 * HTTP Client for LLM-CoPilot SDK
 */

import type {
  CopilotConfig,
  ApiResponse,
  ApiError,
  PaginationParams,
  PaginatedResponse,
} from './types';

/**
 * HTTP method types
 */
type HttpMethod = 'GET' | 'POST' | 'PUT' | 'PATCH' | 'DELETE';

/**
 * Request options
 */
interface RequestOptions<T = unknown> {
  method?: HttpMethod;
  body?: T;
  query?: Record<string, string | number | boolean | undefined>;
  headers?: Record<string, string>;
  signal?: AbortSignal;
  timeout?: number;
}

/**
 * Stream response handler
 */
interface StreamHandler {
  onChunk: (chunk: string) => void;
  onError?: (error: Error) => void;
  onComplete?: () => void;
}

/**
 * SDK Error class
 */
export class CopilotError extends Error {
  constructor(
    message: string,
    public readonly code: string,
    public readonly status?: number,
    public readonly details?: Record<string, unknown>,
    public readonly requestId?: string
  ) {
    super(message);
    this.name = 'CopilotError';
  }

  static fromApiError(error: ApiError, status?: number): CopilotError {
    return new CopilotError(
      error.message,
      error.code,
      status,
      error.details,
      error.requestId
    );
  }
}

/**
 * HTTP Client for making API requests
 */
export class HttpClient {
  private readonly config: Required<
    Pick<CopilotConfig, 'baseUrl' | 'apiKey' | 'timeout' | 'maxRetries'>
  > &
    Pick<CopilotConfig, 'tenantId' | 'headers' | 'debug'>;

  constructor(config: CopilotConfig) {
    this.config = {
      baseUrl: config.baseUrl.replace(/\/+$/, ''),
      apiKey: config.apiKey,
      timeout: config.timeout ?? 30000,
      maxRetries: config.maxRetries ?? 3,
      tenantId: config.tenantId,
      headers: config.headers,
      debug: config.debug,
    };
  }

  /**
   * Build full URL with query parameters
   */
  private buildUrl(
    path: string,
    query?: Record<string, string | number | boolean | undefined>
  ): string {
    const url = new URL(path, this.config.baseUrl);

    if (query) {
      Object.entries(query).forEach(([key, value]) => {
        if (value !== undefined) {
          url.searchParams.append(key, String(value));
        }
      });
    }

    return url.toString();
  }

  /**
   * Build request headers
   */
  private buildHeaders(customHeaders?: Record<string, string>): Headers {
    const headers = new Headers({
      'Content-Type': 'application/json',
      Authorization: `Bearer ${this.config.apiKey}`,
      'User-Agent': '@llm-copilot/sdk',
      ...this.config.headers,
      ...customHeaders,
    });

    if (this.config.tenantId) {
      headers.set('X-Tenant-ID', this.config.tenantId);
    }

    return headers;
  }

  /**
   * Log debug information
   */
  private debug(message: string, data?: unknown): void {
    if (this.config.debug) {
      console.log(`[CopilotSDK] ${message}`, data ?? '');
    }
  }

  /**
   * Execute a request with retry logic
   */
  private async executeWithRetry<T>(
    fn: () => Promise<T>,
    retries: number = this.config.maxRetries
  ): Promise<T> {
    let lastError: Error | undefined;

    for (let attempt = 0; attempt <= retries; attempt++) {
      try {
        return await fn();
      } catch (error) {
        lastError = error as Error;

        // Don't retry on client errors (4xx) except 429 (rate limit)
        if (error instanceof CopilotError) {
          if (
            error.status &&
            error.status >= 400 &&
            error.status < 500 &&
            error.status !== 429
          ) {
            throw error;
          }
        }

        if (attempt < retries) {
          const delay = Math.min(1000 * Math.pow(2, attempt), 10000);
          this.debug(`Retry attempt ${attempt + 1} after ${delay}ms`);
          await new Promise((resolve) => setTimeout(resolve, delay));
        }
      }
    }

    throw lastError;
  }

  /**
   * Make an HTTP request
   */
  async request<T, B = unknown>(
    path: string,
    options: RequestOptions<B> = {}
  ): Promise<ApiResponse<T>> {
    const { method = 'GET', body, query, headers, signal, timeout } = options;

    const url = this.buildUrl(path, query);
    const requestHeaders = this.buildHeaders(headers);

    this.debug(`${method} ${url}`);

    const controller = new AbortController();
    const timeoutId = setTimeout(
      () => controller.abort(),
      timeout ?? this.config.timeout
    );

    try {
      const response = await this.executeWithRetry(async () => {
        const res = await fetch(url, {
          method,
          headers: requestHeaders,
          body: body ? JSON.stringify(body) : undefined,
          signal: signal ?? controller.signal,
        });

        if (!res.ok) {
          const errorBody = await res.json().catch(() => ({}));
          const apiError: ApiError = {
            code: errorBody.code ?? 'UNKNOWN_ERROR',
            message: errorBody.message ?? res.statusText,
            details: errorBody.details,
            requestId: res.headers.get('X-Request-ID') ?? undefined,
          };
          throw CopilotError.fromApiError(apiError, res.status);
        }

        return res;
      });

      const data = await response.json();

      return {
        success: true,
        data: data as T,
        metadata: {
          requestId: response.headers.get('X-Request-ID') ?? '',
          processingTimeMs: parseInt(
            response.headers.get('X-Processing-Time') ?? '0',
            10
          ),
          rateLimit: this.parseRateLimitHeaders(response.headers),
        },
      };
    } catch (error) {
      if (error instanceof CopilotError) {
        throw error;
      }

      if (error instanceof Error && error.name === 'AbortError') {
        throw new CopilotError('Request timeout', 'TIMEOUT', 408);
      }

      throw new CopilotError(
        error instanceof Error ? error.message : 'Unknown error',
        'NETWORK_ERROR'
      );
    } finally {
      clearTimeout(timeoutId);
    }
  }

  /**
   * Parse rate limit headers from response
   */
  private parseRateLimitHeaders(
    headers: Headers
  ): { limit: number; remaining: number; resetAt: Date } | undefined {
    const limit = headers.get('X-RateLimit-Limit');
    const remaining = headers.get('X-RateLimit-Remaining');
    const reset = headers.get('X-RateLimit-Reset');

    if (limit && remaining && reset) {
      return {
        limit: parseInt(limit, 10),
        remaining: parseInt(remaining, 10),
        resetAt: new Date(parseInt(reset, 10) * 1000),
      };
    }

    return undefined;
  }

  /**
   * GET request
   */
  async get<T>(
    path: string,
    query?: Record<string, string | number | boolean | undefined>,
    options?: Omit<RequestOptions, 'method' | 'body' | 'query'>
  ): Promise<ApiResponse<T>> {
    return this.request<T>(path, { ...options, method: 'GET', query });
  }

  /**
   * POST request
   */
  async post<T, B = unknown>(
    path: string,
    body?: B,
    options?: Omit<RequestOptions, 'method' | 'body'>
  ): Promise<ApiResponse<T>> {
    return this.request<T, B>(path, { ...options, method: 'POST', body });
  }

  /**
   * PUT request
   */
  async put<T, B = unknown>(
    path: string,
    body?: B,
    options?: Omit<RequestOptions, 'method' | 'body'>
  ): Promise<ApiResponse<T>> {
    return this.request<T, B>(path, { ...options, method: 'PUT', body });
  }

  /**
   * PATCH request
   */
  async patch<T, B = unknown>(
    path: string,
    body?: B,
    options?: Omit<RequestOptions, 'method' | 'body'>
  ): Promise<ApiResponse<T>> {
    return this.request<T, B>(path, { ...options, method: 'PATCH', body });
  }

  /**
   * DELETE request
   */
  async delete<T>(
    path: string,
    options?: Omit<RequestOptions, 'method' | 'body'>
  ): Promise<ApiResponse<T>> {
    return this.request<T>(path, { ...options, method: 'DELETE' });
  }

  /**
   * Stream request using Server-Sent Events
   */
  async stream(
    path: string,
    body: unknown,
    handler: StreamHandler,
    signal?: AbortSignal
  ): Promise<void> {
    const url = this.buildUrl(path);
    const headers = this.buildHeaders({ Accept: 'text/event-stream' });

    this.debug(`STREAM ${url}`);

    try {
      const response = await fetch(url, {
        method: 'POST',
        headers,
        body: JSON.stringify(body),
        signal,
      });

      if (!response.ok) {
        const errorBody = await response.json().catch(() => ({}));
        throw CopilotError.fromApiError(
          {
            code: errorBody.code ?? 'STREAM_ERROR',
            message: errorBody.message ?? response.statusText,
          },
          response.status
        );
      }

      if (!response.body) {
        throw new CopilotError('No response body', 'NO_BODY', 500);
      }

      const reader = response.body.getReader();
      const decoder = new TextDecoder();

      try {
        while (true) {
          const { done, value } = await reader.read();

          if (done) {
            handler.onComplete?.();
            break;
          }

          const chunk = decoder.decode(value, { stream: true });
          const lines = chunk.split('\n');

          for (const line of lines) {
            if (line.startsWith('data: ')) {
              const data = line.slice(6);
              if (data === '[DONE]') {
                handler.onComplete?.();
                return;
              }
              handler.onChunk(data);
            }
          }
        }
      } finally {
        reader.releaseLock();
      }
    } catch (error) {
      if (error instanceof CopilotError) {
        handler.onError?.(error);
        throw error;
      }

      const copilotError = new CopilotError(
        error instanceof Error ? error.message : 'Stream error',
        'STREAM_ERROR'
      );
      handler.onError?.(copilotError);
      throw copilotError;
    }
  }

  /**
   * Paginated GET request
   */
  async paginate<T>(
    path: string,
    params?: PaginationParams & Record<string, unknown>
  ): Promise<ApiResponse<PaginatedResponse<T>>> {
    const { page = 1, pageSize = 20, cursor, ...rest } = params ?? {};

    return this.get<PaginatedResponse<T>>(path, {
      page,
      page_size: pageSize,
      cursor,
      ...rest,
    });
  }

  /**
   * Iterate through all pages
   */
  async *paginateAll<T>(
    path: string,
    params?: Omit<PaginationParams, 'page' | 'cursor'> &
      Record<string, unknown>
  ): AsyncGenerator<T, void, unknown> {
    let cursor: string | undefined;
    let hasMore = true;

    while (hasMore) {
      const response = await this.paginate<T>(path, { ...params, cursor });

      if (!response.success || !response.data) {
        throw new CopilotError('Pagination failed', 'PAGINATION_ERROR');
      }

      for (const item of response.data.items) {
        yield item;
      }

      hasMore = response.data.hasMore;
      cursor = response.data.nextCursor;
    }
  }
}

export { HttpClient as default };
