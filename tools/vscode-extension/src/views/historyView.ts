/**
 * History Tree View Provider
 */

import * as vscode from 'vscode';
import { CoPilotClient, Conversation } from '../client';

export class HistoryViewProvider implements vscode.TreeDataProvider<ConversationItem> {
    private _onDidChangeTreeData = new vscode.EventEmitter<ConversationItem | undefined>();
    readonly onDidChangeTreeData = this._onDidChangeTreeData.event;

    constructor(private client: CoPilotClient) {}

    refresh(): void {
        this._onDidChangeTreeData.fire(undefined);
    }

    getTreeItem(element: ConversationItem): vscode.TreeItem {
        return element;
    }

    async getChildren(element?: ConversationItem): Promise<ConversationItem[]> {
        if (element) {
            return [];
        }

        try {
            const conversations = await this.client.listConversations();
            return conversations.map((conv) => new ConversationItem(conv, this.client));
        } catch (error) {
            vscode.window.showErrorMessage('Failed to load conversation history');
            return [];
        }
    }
}

class ConversationItem extends vscode.TreeItem {
    constructor(
        private conversation: Conversation,
        private client: CoPilotClient
    ) {
        super(
            conversation.title || `Conversation ${conversation.id.substring(0, 8)}`,
            vscode.TreeItemCollapsibleState.None
        );

        this.tooltip = `${conversation.messageCount} messages\nCreated: ${new Date(conversation.createdAt).toLocaleString()}`;
        this.description = `${conversation.messageCount} messages`;
        this.iconPath = new vscode.ThemeIcon('comment-discussion');

        this.command = {
            command: 'llmCopilot.openConversation',
            title: 'Open Conversation',
            arguments: [conversation.id],
        };

        this.contextValue = 'conversation';
    }
}
