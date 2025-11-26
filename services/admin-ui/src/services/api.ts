/**
 * API Service
 *
 * Centralized API client for communicating with backend services.
 */

import axios, { AxiosInstance, AxiosError } from 'axios';
import type {
  ApiResponse,
  PaginatedResponse,
  Tenant,
  Subscription,
  UsageSummary,
  UsageAggregation,
  Invoice,
  DashboardStats,
  Ticket,
  Incident,
} from '../types';

const API_BASE_URL = import.meta.env.VITE_API_URL || '/api/v1';

class ApiService {
  private client: AxiosInstance;

  constructor() {
    this.client = axios.create({
      baseURL: API_BASE_URL,
      headers: {
        'Content-Type': 'application/json',
      },
    });

    // Request interceptor for auth
    this.client.interceptors.request.use((config) => {
      const token = localStorage.getItem('authToken');
      if (token) {
        config.headers.Authorization = `Bearer ${token}`;
      }
      return config;
    });

    // Response interceptor for error handling
    this.client.interceptors.response.use(
      (response) => response,
      (error: AxiosError) => {
        if (error.response?.status === 401) {
          localStorage.removeItem('authToken');
          window.location.href = '/login';
        }
        return Promise.reject(error);
      }
    );
  }

  // Authentication
  async login(email: string, password: string): Promise<{ token: string; user: any }> {
    const response = await this.client.post('/auth/login', { email, password });
    return response.data.data;
  }

  async logout(): Promise<void> {
    await this.client.post('/auth/logout');
    localStorage.removeItem('authToken');
  }

  // Dashboard
  async getDashboardStats(): Promise<DashboardStats> {
    const response = await this.client.get<ApiResponse<DashboardStats>>('/admin/dashboard');
    return response.data.data;
  }

  // Tenants
  async getTenants(page = 1, pageSize = 20): Promise<PaginatedResponse<Tenant>> {
    const response = await this.client.get<PaginatedResponse<Tenant>>('/admin/tenants', {
      params: { page, pageSize },
    });
    return response.data;
  }

  async getTenant(tenantId: string): Promise<Tenant> {
    const response = await this.client.get<ApiResponse<Tenant>>(`/admin/tenants/${tenantId}`);
    return response.data.data;
  }

  async updateTenant(tenantId: string, data: Partial<Tenant>): Promise<Tenant> {
    const response = await this.client.patch<ApiResponse<Tenant>>(`/admin/tenants/${tenantId}`, data);
    return response.data.data;
  }

  async suspendTenant(tenantId: string): Promise<void> {
    await this.client.post(`/admin/tenants/${tenantId}/suspend`);
  }

  async reactivateTenant(tenantId: string): Promise<void> {
    await this.client.post(`/admin/tenants/${tenantId}/reactivate`);
  }

  // Subscriptions
  async getSubscription(tenantId: string): Promise<Subscription> {
    const response = await this.client.get<ApiResponse<Subscription>>(`/subscriptions/tenant/${tenantId}`);
    return response.data.data;
  }

  async updateSubscription(subscriptionId: string, data: Partial<Subscription>): Promise<Subscription> {
    const response = await this.client.patch<ApiResponse<Subscription>>(`/subscriptions/${subscriptionId}`, data);
    return response.data.data;
  }

  // Usage
  async getUsageSummary(tenantId: string, startDate: string, endDate: string): Promise<UsageSummary> {
    const response = await this.client.get<ApiResponse<UsageSummary>>(`/usage/summary/${tenantId}`, {
      params: { startDate, endDate },
    });
    return response.data.data;
  }

  async getUsageAggregations(
    tenantId: string,
    startDate: string,
    endDate: string,
    groupBy = 'day'
  ): Promise<UsageAggregation[]> {
    const response = await this.client.get<ApiResponse<UsageAggregation[]>>(`/usage/aggregations/${tenantId}`, {
      params: { startDate, endDate, groupBy },
    });
    return response.data.data;
  }

  async getPlatformUsage(startDate: string, endDate: string): Promise<UsageSummary> {
    const response = await this.client.get<ApiResponse<UsageSummary>>('/admin/usage', {
      params: { startDate, endDate },
    });
    return response.data.data;
  }

  // Invoices
  async getInvoices(tenantId: string, limit = 10): Promise<Invoice[]> {
    const response = await this.client.get<ApiResponse<Invoice[]>>(`/invoices/tenant/${tenantId}`, {
      params: { limit },
    });
    return response.data.data;
  }

  async getAllInvoices(page = 1, pageSize = 20): Promise<PaginatedResponse<Invoice>> {
    const response = await this.client.get<PaginatedResponse<Invoice>>('/admin/invoices', {
      params: { page, pageSize },
    });
    return response.data;
  }

  // Support Tickets
  async getTickets(page = 1, pageSize = 20, status?: string): Promise<PaginatedResponse<Ticket>> {
    const response = await this.client.get<PaginatedResponse<Ticket>>('/admin/tickets', {
      params: { page, pageSize, status },
    });
    return response.data;
  }

  async getTicket(ticketId: string): Promise<Ticket> {
    const response = await this.client.get<ApiResponse<Ticket>>(`/admin/tickets/${ticketId}`);
    return response.data.data;
  }

  async updateTicket(ticketId: string, data: Partial<Ticket>): Promise<Ticket> {
    const response = await this.client.patch<ApiResponse<Ticket>>(`/admin/tickets/${ticketId}`, data);
    return response.data.data;
  }

  async assignTicket(ticketId: string, assigneeId: string): Promise<void> {
    await this.client.post(`/admin/tickets/${ticketId}/assign`, { assigneeId });
  }

  // Incidents
  async getIncidents(page = 1, pageSize = 20): Promise<PaginatedResponse<Incident>> {
    const response = await this.client.get<PaginatedResponse<Incident>>('/admin/incidents', {
      params: { page, pageSize },
    });
    return response.data;
  }

  async getIncident(incidentId: string): Promise<Incident> {
    const response = await this.client.get<ApiResponse<Incident>>(`/admin/incidents/${incidentId}`);
    return response.data.data;
  }

  async createIncident(data: Partial<Incident>): Promise<Incident> {
    const response = await this.client.post<ApiResponse<Incident>>('/admin/incidents', data);
    return response.data.data;
  }

  async updateIncident(incidentId: string, data: Partial<Incident>): Promise<Incident> {
    const response = await this.client.patch<ApiResponse<Incident>>(`/admin/incidents/${incidentId}`, data);
    return response.data.data;
  }

  async addIncidentUpdate(incidentId: string, message: string, status: string): Promise<void> {
    await this.client.post(`/admin/incidents/${incidentId}/updates`, { message, status });
  }

  // System Health
  async getSystemHealth(): Promise<any> {
    const response = await this.client.get('/health');
    return response.data;
  }

  async getServiceStatus(): Promise<any[]> {
    const response = await this.client.get('/admin/services/status');
    return response.data.data;
  }
}

export const api = new ApiService();
export default api;
