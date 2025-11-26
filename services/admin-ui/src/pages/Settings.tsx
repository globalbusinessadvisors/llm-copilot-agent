/**
 * Settings Page
 *
 * System configuration and admin settings.
 */

import React, { useState } from 'react';

interface SettingSection {
  id: string;
  title: string;
  description: string;
}

const sections: SettingSection[] = [
  { id: 'general', title: 'General', description: 'Basic system settings' },
  { id: 'billing', title: 'Billing', description: 'Payment and pricing configuration' },
  { id: 'notifications', title: 'Notifications', description: 'Alert and notification preferences' },
  { id: 'security', title: 'Security', description: 'Security and authentication settings' },
  { id: 'api', title: 'API', description: 'API rate limits and configuration' },
  { id: 'integrations', title: 'Integrations', description: 'Third-party service connections' },
];

export function Settings() {
  const [activeSection, setActiveSection] = useState('general');
  const [saved, setSaved] = useState(false);

  const handleSave = () => {
    setSaved(true);
    setTimeout(() => setSaved(false), 3000);
  };

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-gray-900">Settings</h1>
          <p className="text-gray-500">Manage system configuration</p>
        </div>
        {saved && (
          <div className="bg-green-100 text-green-700 px-4 py-2 rounded-lg flex items-center space-x-2">
            <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 13l4 4L19 7" />
            </svg>
            <span>Settings saved</span>
          </div>
        )}
      </div>

      <div className="flex gap-6">
        {/* Sidebar */}
        <div className="w-64 shrink-0">
          <nav className="space-y-1">
            {sections.map((section) => (
              <button
                key={section.id}
                onClick={() => setActiveSection(section.id)}
                className={`w-full text-left px-4 py-3 rounded-lg transition-colors ${
                  activeSection === section.id
                    ? 'bg-primary-50 text-primary-700'
                    : 'text-gray-600 hover:bg-gray-50'
                }`}
              >
                <p className="font-medium">{section.title}</p>
                <p className="text-sm text-gray-500">{section.description}</p>
              </button>
            ))}
          </nav>
        </div>

        {/* Content */}
        <div className="flex-1">
          {activeSection === 'general' && (
            <div className="card space-y-6">
              <h2 className="text-lg font-semibold">General Settings</h2>

              <div>
                <label className="label">Platform Name</label>
                <input type="text" className="input" defaultValue="LLM CoPilot" />
              </div>

              <div>
                <label className="label">Support Email</label>
                <input type="email" className="input" defaultValue="support@llmcopilot.com" />
              </div>

              <div>
                <label className="label">Default Timezone</label>
                <select className="input">
                  <option>UTC</option>
                  <option>America/New_York</option>
                  <option>America/Los_Angeles</option>
                  <option>Europe/London</option>
                </select>
              </div>

              <div className="flex items-center justify-between py-4 border-t">
                <div>
                  <p className="font-medium">Maintenance Mode</p>
                  <p className="text-sm text-gray-500">Disable access for non-admin users</p>
                </div>
                <label className="relative inline-flex items-center cursor-pointer">
                  <input type="checkbox" className="sr-only peer" />
                  <div className="w-11 h-6 bg-gray-200 peer-focus:outline-none peer-focus:ring-4 peer-focus:ring-primary-300 rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-primary-600"></div>
                </label>
              </div>

              <button onClick={handleSave} className="btn btn-primary">Save Changes</button>
            </div>
          )}

          {activeSection === 'billing' && (
            <div className="card space-y-6">
              <h2 className="text-lg font-semibold">Billing Settings</h2>

              <div>
                <label className="label">Stripe API Key</label>
                <input type="password" className="input" defaultValue="sk_live_..." />
              </div>

              <div>
                <label className="label">Stripe Webhook Secret</label>
                <input type="password" className="input" defaultValue="whsec_..." />
              </div>

              <div>
                <label className="label">Default Currency</label>
                <select className="input">
                  <option value="usd">USD ($)</option>
                  <option value="eur">EUR (€)</option>
                  <option value="gbp">GBP (£)</option>
                </select>
              </div>

              <div>
                <label className="label">Invoice Due Days</label>
                <input type="number" className="input w-32" defaultValue="30" />
              </div>

              <div className="flex items-center justify-between py-4 border-t">
                <div>
                  <p className="font-medium">Auto-charge Failed Payments</p>
                  <p className="text-sm text-gray-500">Automatically retry failed payment attempts</p>
                </div>
                <label className="relative inline-flex items-center cursor-pointer">
                  <input type="checkbox" className="sr-only peer" defaultChecked />
                  <div className="w-11 h-6 bg-gray-200 peer-focus:outline-none peer-focus:ring-4 peer-focus:ring-primary-300 rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-primary-600"></div>
                </label>
              </div>

              <button onClick={handleSave} className="btn btn-primary">Save Changes</button>
            </div>
          )}

          {activeSection === 'notifications' && (
            <div className="card space-y-6">
              <h2 className="text-lg font-semibold">Notification Settings</h2>

              <div className="space-y-4">
                {[
                  { label: 'New tenant signup', key: 'tenant_signup' },
                  { label: 'Subscription changes', key: 'subscription_change' },
                  { label: 'Payment failures', key: 'payment_failure' },
                  { label: 'Quota warnings (80%)', key: 'quota_warning' },
                  { label: 'Quota exceeded', key: 'quota_exceeded' },
                  { label: 'Support tickets', key: 'support_ticket' },
                  { label: 'System incidents', key: 'incident' },
                ].map((item) => (
                  <div key={item.key} className="flex items-center justify-between py-2">
                    <span>{item.label}</span>
                    <div className="flex space-x-4">
                      <label className="flex items-center space-x-2">
                        <input type="checkbox" className="rounded" defaultChecked />
                        <span className="text-sm text-gray-500">Email</span>
                      </label>
                      <label className="flex items-center space-x-2">
                        <input type="checkbox" className="rounded" />
                        <span className="text-sm text-gray-500">Slack</span>
                      </label>
                    </div>
                  </div>
                ))}
              </div>

              <div className="border-t pt-6">
                <label className="label">Slack Webhook URL</label>
                <input type="url" className="input" placeholder="https://hooks.slack.com/..." />
              </div>

              <button onClick={handleSave} className="btn btn-primary">Save Changes</button>
            </div>
          )}

          {activeSection === 'security' && (
            <div className="card space-y-6">
              <h2 className="text-lg font-semibold">Security Settings</h2>

              <div>
                <label className="label">Session Timeout (minutes)</label>
                <input type="number" className="input w-32" defaultValue="60" />
              </div>

              <div>
                <label className="label">JWT Token Expiry (hours)</label>
                <input type="number" className="input w-32" defaultValue="24" />
              </div>

              <div className="flex items-center justify-between py-4 border-t">
                <div>
                  <p className="font-medium">Require 2FA for Admins</p>
                  <p className="text-sm text-gray-500">All admin users must enable 2FA</p>
                </div>
                <label className="relative inline-flex items-center cursor-pointer">
                  <input type="checkbox" className="sr-only peer" defaultChecked />
                  <div className="w-11 h-6 bg-gray-200 peer-focus:outline-none peer-focus:ring-4 peer-focus:ring-primary-300 rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-primary-600"></div>
                </label>
              </div>

              <div className="flex items-center justify-between py-4 border-t">
                <div>
                  <p className="font-medium">IP Allowlist</p>
                  <p className="text-sm text-gray-500">Restrict admin access to specific IPs</p>
                </div>
                <label className="relative inline-flex items-center cursor-pointer">
                  <input type="checkbox" className="sr-only peer" />
                  <div className="w-11 h-6 bg-gray-200 peer-focus:outline-none peer-focus:ring-4 peer-focus:ring-primary-300 rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-primary-600"></div>
                </label>
              </div>

              <button onClick={handleSave} className="btn btn-primary">Save Changes</button>
            </div>
          )}

          {activeSection === 'api' && (
            <div className="card space-y-6">
              <h2 className="text-lg font-semibold">API Settings</h2>

              <div className="grid grid-cols-2 gap-4">
                <div>
                  <label className="label">Default Rate Limit (req/min)</label>
                  <input type="number" className="input" defaultValue="100" />
                </div>
                <div>
                  <label className="label">Burst Limit</label>
                  <input type="number" className="input" defaultValue="200" />
                </div>
              </div>

              <div>
                <label className="label">Request Timeout (seconds)</label>
                <input type="number" className="input w-32" defaultValue="30" />
              </div>

              <div>
                <label className="label">Max Request Body Size (MB)</label>
                <input type="number" className="input w-32" defaultValue="10" />
              </div>

              <div className="flex items-center justify-between py-4 border-t">
                <div>
                  <p className="font-medium">API Versioning</p>
                  <p className="text-sm text-gray-500">Current version: v1</p>
                </div>
                <span className="badge badge-info">v1</span>
              </div>

              <button onClick={handleSave} className="btn btn-primary">Save Changes</button>
            </div>
          )}

          {activeSection === 'integrations' && (
            <div className="card space-y-6">
              <h2 className="text-lg font-semibold">Integrations</h2>

              <div className="space-y-4">
                {[
                  { name: 'Stripe', status: 'connected', icon: '💳' },
                  { name: 'Slack', status: 'disconnected', icon: '💬' },
                  { name: 'PagerDuty', status: 'connected', icon: '🔔' },
                  { name: 'Datadog', status: 'disconnected', icon: '📊' },
                  { name: 'AWS S3', status: 'connected', icon: '☁️' },
                ].map((integration) => (
                  <div key={integration.name} className="flex items-center justify-between p-4 border rounded-lg">
                    <div className="flex items-center space-x-3">
                      <span className="text-2xl">{integration.icon}</span>
                      <div>
                        <p className="font-medium">{integration.name}</p>
                        <p className={`text-sm ${integration.status === 'connected' ? 'text-green-600' : 'text-gray-500'}`}>
                          {integration.status === 'connected' ? 'Connected' : 'Not connected'}
                        </p>
                      </div>
                    </div>
                    <button className={`btn ${integration.status === 'connected' ? 'btn-secondary' : 'btn-primary'}`}>
                      {integration.status === 'connected' ? 'Configure' : 'Connect'}
                    </button>
                  </div>
                ))}
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

export default Settings;
