/**
 * Authentication Hook
 *
 * Manages authentication state and provides auth methods.
 */

import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import type { User, AuthState } from '../types';
import api from '../services/api';

interface AuthStore extends AuthState {
  login: (email: string, password: string) => Promise<void>;
  logout: () => Promise<void>;
  setUser: (user: User | null) => void;
  setToken: (token: string | null) => void;
}

export const useAuth = create<AuthStore>()(
  persist(
    (set) => ({
      user: null,
      token: null,
      isAuthenticated: false,
      isLoading: false,

      login: async (email: string, password: string) => {
        set({ isLoading: true });
        try {
          const { token, user } = await api.login(email, password);
          localStorage.setItem('authToken', token);
          set({ user, token, isAuthenticated: true, isLoading: false });
        } catch (error) {
          set({ isLoading: false });
          throw error;
        }
      },

      logout: async () => {
        try {
          await api.logout();
        } finally {
          localStorage.removeItem('authToken');
          set({ user: null, token: null, isAuthenticated: false });
        }
      },

      setUser: (user) => set({ user, isAuthenticated: !!user }),
      setToken: (token) => {
        if (token) {
          localStorage.setItem('authToken', token);
        } else {
          localStorage.removeItem('authToken');
        }
        set({ token });
      },
    }),
    {
      name: 'auth-storage',
      partialize: (state) => ({ token: state.token, user: state.user }),
    }
  )
);

export default useAuth;
