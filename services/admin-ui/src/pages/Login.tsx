/**
 * Login Page
 *
 * Authentication page for admin users.
 */

import React, { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { useAuth } from '../hooks/useAuth';

export function Login() {
  const navigate = useNavigate();
  const { login, isLoading } = useAuth();
  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');
  const [error, setError] = useState('');

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError('');

    try {
      await login(email, password);
      navigate('/');
    } catch (err) {
      setError('Invalid email or password');
    }
  };

  // For demo purposes, bypass login
  const handleDemoLogin = () => {
    useAuth.setState({
      user: {
        id: 'demo-user',
        email: 'admin@example.com',
        name: 'Demo Admin',
        role: 'admin',
      },
      token: 'demo-token',
      isAuthenticated: true,
    });
    navigate('/');
  };

  return (
    <div className="min-h-screen flex items-center justify-center bg-gray-100 px-4">
      <div className="max-w-md w-full">
        <div className="text-center mb-8">
          <h1 className="text-3xl font-bold text-primary-600">LLM CoPilot</h1>
          <p className="text-gray-600 mt-2">Admin Dashboard</p>
        </div>

        <div className="card">
          <h2 className="text-xl font-semibold text-center mb-6">Sign In</h2>

          {error && (
            <div className="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded-lg mb-4">
              {error}
            </div>
          )}

          <form onSubmit={handleSubmit} className="space-y-4">
            <div>
              <label className="label">Email</label>
              <input
                type="email"
                className="input"
                placeholder="admin@example.com"
                value={email}
                onChange={(e) => setEmail(e.target.value)}
                required
              />
            </div>

            <div>
              <label className="label">Password</label>
              <input
                type="password"
                className="input"
                placeholder="Enter your password"
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                required
              />
            </div>

            <div className="flex items-center justify-between">
              <label className="flex items-center">
                <input type="checkbox" className="mr-2" />
                <span className="text-sm text-gray-600">Remember me</span>
              </label>
              <a href="#" className="text-sm text-primary-600 hover:underline">
                Forgot password?
              </a>
            </div>

            <button
              type="submit"
              className="btn btn-primary w-full"
              disabled={isLoading}
            >
              {isLoading ? 'Signing in...' : 'Sign In'}
            </button>
          </form>

          <div className="mt-6 pt-6 border-t border-gray-200">
            <button
              onClick={handleDemoLogin}
              className="btn btn-secondary w-full"
            >
              Demo Login (No Auth Required)
            </button>
          </div>
        </div>

        <p className="text-center text-gray-500 text-sm mt-6">
          &copy; {new Date().getFullYear()} LLM CoPilot. All rights reserved.
        </p>
      </div>
    </div>
  );
}

export default Login;
