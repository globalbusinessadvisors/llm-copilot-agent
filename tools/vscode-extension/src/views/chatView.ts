/**
 * Chat View Provider for the LLM CoPilot sidebar
 */

import * as vscode from 'vscode';
import { CoPilotClient } from '../client';

export class ChatViewProvider implements vscode.WebviewViewProvider {
    public static readonly viewType = 'llmCopilot.chat';
    private view?: vscode.WebviewView;

    constructor(
        private readonly extensionUri: vscode.Uri,
        private readonly client: CoPilotClient
    ) {}

    public resolveWebviewView(
        webviewView: vscode.WebviewView,
        context: vscode.WebviewViewResolveContext,
        token: vscode.CancellationToken
    ) {
        this.view = webviewView;

        webviewView.webview.options = {
            enableScripts: true,
            localResourceRoots: [this.extensionUri],
        };

        webviewView.webview.html = this.getHtml();

        webviewView.webview.onDidReceiveMessage(async (data) => {
            switch (data.type) {
                case 'sendMessage':
                    await this.handleSendMessage(data.message);
                    break;
                case 'newConversation':
                    await this.handleNewConversation();
                    break;
                case 'checkHealth':
                    await this.handleHealthCheck();
                    break;
            }
        });
    }

    public focus() {
        if (this.view) {
            this.view.show(true);
        }
    }

    private async handleSendMessage(message: string) {
        if (!this.view) return;

        // Show user message
        this.view.webview.postMessage({
            type: 'addMessage',
            role: 'user',
            content: message,
        });

        // Show loading
        this.view.webview.postMessage({ type: 'showLoading' });

        try {
            const response = await this.client.chat(message, { includeContext: true });

            this.view.webview.postMessage({
                type: 'addMessage',
                role: 'assistant',
                content: response,
            });
        } catch (error: any) {
            this.view.webview.postMessage({
                type: 'showError',
                message: error.message,
            });
        }

        this.view.webview.postMessage({ type: 'hideLoading' });
    }

    private async handleNewConversation() {
        this.client.setCurrentConversation(null);
        if (this.view) {
            this.view.webview.postMessage({ type: 'clearMessages' });
        }
    }

    private async handleHealthCheck() {
        const isHealthy = await this.client.healthCheck();
        if (this.view) {
            this.view.webview.postMessage({
                type: 'healthStatus',
                status: isHealthy ? 'connected' : 'disconnected',
            });
        }
    }

    private getHtml(): string {
        return `<!DOCTYPE html>
        <html>
        <head>
            <meta charset="UTF-8">
            <meta name="viewport" content="width=device-width, initial-scale=1.0">
            <style>
                * {
                    box-sizing: border-box;
                    margin: 0;
                    padding: 0;
                }
                body {
                    font-family: var(--vscode-font-family);
                    font-size: var(--vscode-font-size);
                    color: var(--vscode-foreground);
                    background: var(--vscode-sideBar-background);
                    display: flex;
                    flex-direction: column;
                    height: 100vh;
                }
                .header {
                    display: flex;
                    justify-content: space-between;
                    align-items: center;
                    padding: 8px;
                    border-bottom: 1px solid var(--vscode-panel-border);
                }
                .header-title {
                    font-weight: 600;
                    font-size: 0.9em;
                }
                .status {
                    font-size: 0.8em;
                    padding: 2px 6px;
                    border-radius: 4px;
                }
                .status.connected {
                    background: var(--vscode-testing-iconPassed);
                    color: var(--vscode-testing-message);
                }
                .status.disconnected {
                    background: var(--vscode-testing-iconFailed);
                    color: white;
                }
                .messages {
                    flex: 1;
                    overflow-y: auto;
                    padding: 8px;
                }
                .message {
                    margin-bottom: 12px;
                    padding: 8px 12px;
                    border-radius: 8px;
                    max-width: 90%;
                }
                .message.user {
                    background: var(--vscode-button-background);
                    color: var(--vscode-button-foreground);
                    margin-left: auto;
                }
                .message.assistant {
                    background: var(--vscode-editor-inactiveSelectionBackground);
                    margin-right: auto;
                }
                .message pre {
                    background: var(--vscode-textBlockQuote-background);
                    padding: 8px;
                    border-radius: 4px;
                    overflow-x: auto;
                    margin: 8px 0;
                }
                .message code {
                    font-family: var(--vscode-editor-font-family);
                    font-size: 0.9em;
                }
                .loading {
                    display: none;
                    text-align: center;
                    padding: 8px;
                    color: var(--vscode-descriptionForeground);
                }
                .loading.visible {
                    display: block;
                }
                .input-area {
                    padding: 8px;
                    border-top: 1px solid var(--vscode-panel-border);
                }
                .input-wrapper {
                    display: flex;
                    gap: 4px;
                }
                textarea {
                    flex: 1;
                    resize: none;
                    padding: 8px;
                    border: 1px solid var(--vscode-input-border);
                    background: var(--vscode-input-background);
                    color: var(--vscode-input-foreground);
                    border-radius: 4px;
                    font-family: inherit;
                    font-size: inherit;
                    min-height: 60px;
                }
                textarea:focus {
                    outline: 1px solid var(--vscode-focusBorder);
                }
                button {
                    padding: 8px 16px;
                    background: var(--vscode-button-background);
                    color: var(--vscode-button-foreground);
                    border: none;
                    border-radius: 4px;
                    cursor: pointer;
                }
                button:hover {
                    background: var(--vscode-button-hoverBackground);
                }
                button.secondary {
                    background: var(--vscode-button-secondaryBackground);
                    color: var(--vscode-button-secondaryForeground);
                }
                .actions {
                    display: flex;
                    gap: 4px;
                    margin-top: 4px;
                }
                .actions button {
                    flex: 1;
                    padding: 4px 8px;
                    font-size: 0.9em;
                }
                .empty-state {
                    text-align: center;
                    padding: 20px;
                    color: var(--vscode-descriptionForeground);
                }
                .error {
                    background: var(--vscode-inputValidation-errorBackground);
                    border: 1px solid var(--vscode-inputValidation-errorBorder);
                    color: var(--vscode-inputValidation-errorForeground);
                    padding: 8px;
                    border-radius: 4px;
                    margin: 8px;
                }
            </style>
        </head>
        <body>
            <div class="header">
                <span class="header-title">LLM CoPilot</span>
                <span class="status" id="status">Checking...</span>
            </div>

            <div class="messages" id="messages">
                <div class="empty-state" id="empty-state">
                    Start a conversation by typing a message below.
                </div>
            </div>

            <div class="loading" id="loading">
                <span>Thinking...</span>
            </div>

            <div id="error-container"></div>

            <div class="input-area">
                <div class="input-wrapper">
                    <textarea
                        id="input"
                        placeholder="Ask me anything..."
                        rows="3"
                    ></textarea>
                </div>
                <div class="actions">
                    <button id="send-btn">Send</button>
                    <button id="new-btn" class="secondary">New Chat</button>
                </div>
            </div>

            <script>
                const vscode = acquireVsCodeApi();
                const messagesEl = document.getElementById('messages');
                const emptyState = document.getElementById('empty-state');
                const loadingEl = document.getElementById('loading');
                const errorContainer = document.getElementById('error-container');
                const inputEl = document.getElementById('input');
                const statusEl = document.getElementById('status');

                // Check health on load
                vscode.postMessage({ type: 'checkHealth' });

                // Send message
                document.getElementById('send-btn').addEventListener('click', sendMessage);
                inputEl.addEventListener('keydown', (e) => {
                    if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) {
                        sendMessage();
                    }
                });

                function sendMessage() {
                    const message = inputEl.value.trim();
                    if (!message) return;

                    vscode.postMessage({ type: 'sendMessage', message });
                    inputEl.value = '';
                }

                // New conversation
                document.getElementById('new-btn').addEventListener('click', () => {
                    vscode.postMessage({ type: 'newConversation' });
                });

                // Handle messages from extension
                window.addEventListener('message', (event) => {
                    const data = event.data;

                    switch (data.type) {
                        case 'addMessage':
                            addMessage(data.role, data.content);
                            break;
                        case 'showLoading':
                            loadingEl.classList.add('visible');
                            break;
                        case 'hideLoading':
                            loadingEl.classList.remove('visible');
                            break;
                        case 'clearMessages':
                            messagesEl.innerHTML = '';
                            emptyState.style.display = 'block';
                            messagesEl.appendChild(emptyState);
                            break;
                        case 'showError':
                            showError(data.message);
                            break;
                        case 'healthStatus':
                            statusEl.textContent = data.status === 'connected' ? 'Connected' : 'Disconnected';
                            statusEl.className = 'status ' + data.status;
                            break;
                    }
                });

                function addMessage(role, content) {
                    emptyState.style.display = 'none';

                    const messageEl = document.createElement('div');
                    messageEl.className = 'message ' + role;
                    messageEl.innerHTML = formatMessage(content);
                    messagesEl.appendChild(messageEl);
                    messagesEl.scrollTop = messagesEl.scrollHeight;
                }

                function formatMessage(content) {
                    // Convert code blocks
                    content = content.replace(/\`\`\`(\\w+)?\\n([\\s\\S]*?)\`\`\`/g,
                        '<pre><code>$2</code></pre>');
                    // Convert inline code
                    content = content.replace(/\`([^\`]+)\`/g, '<code>$1</code>');
                    // Convert newlines
                    content = content.replace(/\\n/g, '<br>');
                    return content;
                }

                function showError(message) {
                    const errorEl = document.createElement('div');
                    errorEl.className = 'error';
                    errorEl.textContent = message;
                    errorContainer.appendChild(errorEl);

                    setTimeout(() => {
                        errorEl.remove();
                    }, 5000);
                }
            </script>
        </body>
        </html>`;
    }
}
