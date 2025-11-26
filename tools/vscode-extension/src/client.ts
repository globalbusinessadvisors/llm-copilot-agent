/**
 * CoPilot API Client for VS Code Extension
 */

import * as vscode from 'vscode';
import * as https from 'https';
import * as http from 'http';

export interface ChatOptions {
    includeContext?: boolean;
    conversationId?: string;
    model?: string;
    temperature?: number;
    maxTokens?: number;
}

export interface Conversation {
    id: string;
    title: string;
    messageCount: number;
    createdAt: string;
    updatedAt: string;
}

export class CoPilotClient {
    private context: vscode.ExtensionContext;
    private currentConversationId: string | null = null;

    constructor(context: vscode.ExtensionContext) {
        this.context = context;
    }

    private getConfig() {
        return vscode.workspace.getConfiguration('llmCopilot');
    }

    private async request<T>(
        method: string,
        path: string,
        body?: object
    ): Promise<T> {
        const config = this.getConfig();
        const serverUrl = config.get<string>('serverUrl', 'http://localhost:8080');
        const apiKey = config.get<string>('apiKey', '');

        const url = new URL(path, serverUrl);
        const isHttps = url.protocol === 'https:';
        const httpModule = isHttps ? https : http;

        const options = {
            hostname: url.hostname,
            port: url.port || (isHttps ? 443 : 80),
            path: url.pathname + url.search,
            method,
            headers: {
                'Content-Type': 'application/json',
                'Accept': 'application/json',
                ...(apiKey ? { 'X-API-Key': apiKey } : {}),
            },
        };

        return new Promise((resolve, reject) => {
            const req = httpModule.request(options, (res) => {
                let data = '';
                res.on('data', (chunk) => (data += chunk));
                res.on('end', () => {
                    try {
                        if (res.statusCode && res.statusCode >= 400) {
                            const error = JSON.parse(data);
                            reject(new Error(error.message || `HTTP ${res.statusCode}`));
                        } else {
                            resolve(JSON.parse(data) as T);
                        }
                    } catch (e) {
                        reject(new Error(`Failed to parse response: ${data}`));
                    }
                });
            });

            req.on('error', reject);

            if (body) {
                req.write(JSON.stringify(body));
            }
            req.end();
        });
    }

    async chat(message: string, options: ChatOptions = {}): Promise<string> {
        const config = this.getConfig();

        // Create conversation if needed
        if (!this.currentConversationId) {
            const conv = await this.createConversation();
            this.currentConversationId = conv.id;
        }

        // Build context
        let contextMessage = message;
        if (options.includeContext && config.get<boolean>('includeFileContext', true)) {
            const editor = vscode.window.activeTextEditor;
            if (editor) {
                const fileContent = editor.document.getText();
                const fileName = editor.document.fileName;
                contextMessage = `Current file: ${fileName}\n\n${message}\n\nFile content:\n${fileContent}`;
            }
        }

        // Send message
        const response = await this.request<{ content: string }>(
            'POST',
            `/api/v1/conversations/${this.currentConversationId}/messages`,
            {
                content: contextMessage,
                role: 'user',
                metadata: {
                    model: options.model || config.get('defaultModel'),
                    temperature: options.temperature || config.get('temperature'),
                    max_tokens: options.maxTokens || config.get('maxTokens'),
                },
            }
        );

        return response.content;
    }

    async createConversation(title?: string): Promise<Conversation> {
        const response = await this.request<Conversation>(
            'POST',
            '/api/v1/conversations',
            { title: title || `VS Code - ${new Date().toLocaleDateString()}` }
        );
        return response;
    }

    async listConversations(): Promise<Conversation[]> {
        const response = await this.request<{ items: Conversation[] }>(
            'GET',
            '/api/v1/conversations?limit=50'
        );
        return response.items;
    }

    async getConversation(id: string): Promise<Conversation> {
        return this.request<Conversation>('GET', `/api/v1/conversations/${id}`);
    }

    async deleteConversation(id: string): Promise<void> {
        await this.request<void>('DELETE', `/api/v1/conversations/${id}`);
    }

    setCurrentConversation(id: string | null) {
        this.currentConversationId = id;
    }

    getCurrentConversationId(): string | null {
        return this.currentConversationId;
    }

    async healthCheck(): Promise<boolean> {
        try {
            await this.request<{ status: string }>('GET', '/health');
            return true;
        } catch {
            return false;
        }
    }

    dispose() {
        // Cleanup if needed
    }
}
