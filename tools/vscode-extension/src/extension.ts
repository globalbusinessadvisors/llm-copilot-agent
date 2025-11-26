/**
 * LLM CoPilot VS Code Extension
 *
 * Main extension entry point providing AI-powered code assistance.
 */

import * as vscode from 'vscode';
import { CoPilotClient } from './client';
import { ChatViewProvider } from './views/chatView';
import { HistoryViewProvider } from './views/historyView';

let client: CoPilotClient;

export function activate(context: vscode.ExtensionContext) {
    console.log('LLM CoPilot extension is now active');

    // Initialize client
    client = new CoPilotClient(context);

    // Register chat view provider
    const chatViewProvider = new ChatViewProvider(context.extensionUri, client);
    context.subscriptions.push(
        vscode.window.registerWebviewViewProvider('llmCopilot.chat', chatViewProvider)
    );

    // Register history view provider
    const historyViewProvider = new HistoryViewProvider(client);
    context.subscriptions.push(
        vscode.window.registerTreeDataProvider('llmCopilot.history', historyViewProvider)
    );

    // Register commands
    const commands = [
        vscode.commands.registerCommand('llmCopilot.startChat', () => startChat(chatViewProvider)),
        vscode.commands.registerCommand('llmCopilot.explainCode', () => explainCode(client)),
        vscode.commands.registerCommand('llmCopilot.refactorCode', () => refactorCode(client)),
        vscode.commands.registerCommand('llmCopilot.generateTests', () => generateTests(client)),
        vscode.commands.registerCommand('llmCopilot.addDocumentation', () => addDocumentation(client)),
        vscode.commands.registerCommand('llmCopilot.findBugs', () => findBugs(client)),
        vscode.commands.registerCommand('llmCopilot.configure', () => configureSettings()),
        vscode.commands.registerCommand('llmCopilot.showHistory', () => showHistory(historyViewProvider)),
    ];

    context.subscriptions.push(...commands);

    // Status bar item
    const statusBar = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Right, 100);
    statusBar.text = '$(hubot) CoPilot';
    statusBar.tooltip = 'LLM CoPilot - Click to start chat';
    statusBar.command = 'llmCopilot.startChat';
    statusBar.show();
    context.subscriptions.push(statusBar);
}

export function deactivate() {
    client?.dispose();
}

// ===========================================
// Command Implementations
// ===========================================

async function startChat(chatViewProvider: ChatViewProvider) {
    await vscode.commands.executeCommand('workbench.view.extension.llmCopilot');
    chatViewProvider.focus();
}

async function explainCode(client: CoPilotClient) {
    const editor = vscode.window.activeTextEditor;
    if (!editor) {
        vscode.window.showWarningMessage('No editor is active');
        return;
    }

    const selection = editor.selection;
    const text = editor.document.getText(selection);

    if (!text) {
        vscode.window.showWarningMessage('No code selected');
        return;
    }

    await executeWithProgress('Explaining code...', async () => {
        const response = await client.chat(
            `Please explain the following code:\n\n\`\`\`${getLanguageId(editor)}\n${text}\n\`\`\``,
            { includeContext: true }
        );
        showResponsePanel('Code Explanation', response);
    });
}

async function refactorCode(client: CoPilotClient) {
    const editor = vscode.window.activeTextEditor;
    if (!editor) {
        vscode.window.showWarningMessage('No editor is active');
        return;
    }

    const selection = editor.selection;
    const text = editor.document.getText(selection);

    if (!text) {
        vscode.window.showWarningMessage('No code selected');
        return;
    }

    const options = await vscode.window.showQuickPick([
        { label: 'Improve readability', value: 'readability' },
        { label: 'Improve performance', value: 'performance' },
        { label: 'Add error handling', value: 'error-handling' },
        { label: 'Make more concise', value: 'concise' },
        { label: 'Follow best practices', value: 'best-practices' },
    ], { placeHolder: 'Select refactoring goal' });

    if (!options) {
        return;
    }

    await executeWithProgress('Refactoring code...', async () => {
        const response = await client.chat(
            `Please refactor the following code to ${options.label.toLowerCase()}. Provide the refactored code with explanations:\n\n\`\`\`${getLanguageId(editor)}\n${text}\n\`\`\``,
            { includeContext: true }
        );
        showResponsePanel('Code Refactoring', response, true);
    });
}

async function generateTests(client: CoPilotClient) {
    const editor = vscode.window.activeTextEditor;
    if (!editor) {
        vscode.window.showWarningMessage('No editor is active');
        return;
    }

    const selection = editor.selection;
    const text = editor.document.getText(selection);

    if (!text) {
        vscode.window.showWarningMessage('No code selected');
        return;
    }

    const framework = await vscode.window.showQuickPick([
        { label: 'Jest', value: 'jest' },
        { label: 'Mocha', value: 'mocha' },
        { label: 'pytest', value: 'pytest' },
        { label: 'JUnit', value: 'junit' },
        { label: 'Auto-detect', value: 'auto' },
    ], { placeHolder: 'Select test framework' });

    if (!framework) {
        return;
    }

    await executeWithProgress('Generating tests...', async () => {
        const response = await client.chat(
            `Generate comprehensive unit tests for the following code using ${framework.label}. Include edge cases and common scenarios:\n\n\`\`\`${getLanguageId(editor)}\n${text}\n\`\`\``,
            { includeContext: true }
        );
        showResponsePanel('Generated Tests', response, true);
    });
}

async function addDocumentation(client: CoPilotClient) {
    const editor = vscode.window.activeTextEditor;
    if (!editor) {
        vscode.window.showWarningMessage('No editor is active');
        return;
    }

    const selection = editor.selection;
    const text = editor.document.getText(selection);

    if (!text) {
        vscode.window.showWarningMessage('No code selected');
        return;
    }

    await executeWithProgress('Generating documentation...', async () => {
        const response = await client.chat(
            `Add comprehensive documentation (docstrings, JSDoc, etc.) to the following code. Use the appropriate format for the language:\n\n\`\`\`${getLanguageId(editor)}\n${text}\n\`\`\``,
            { includeContext: true }
        );
        showResponsePanel('Documented Code', response, true);
    });
}

async function findBugs(client: CoPilotClient) {
    const editor = vscode.window.activeTextEditor;
    if (!editor) {
        vscode.window.showWarningMessage('No editor is active');
        return;
    }

    const selection = editor.selection;
    const text = editor.document.getText(selection);

    if (!text) {
        vscode.window.showWarningMessage('No code selected');
        return;
    }

    await executeWithProgress('Analyzing code for bugs...', async () => {
        const response = await client.chat(
            `Analyze the following code for potential bugs, security issues, and improvements. Provide specific line numbers and explanations:\n\n\`\`\`${getLanguageId(editor)}\n${text}\n\`\`\``,
            { includeContext: true }
        );
        showResponsePanel('Bug Analysis', response);
    });
}

function configureSettings() {
    vscode.commands.executeCommand('workbench.action.openSettings', 'llmCopilot');
}

function showHistory(historyViewProvider: HistoryViewProvider) {
    historyViewProvider.refresh();
    vscode.commands.executeCommand('workbench.view.extension.llmCopilot');
}

// ===========================================
// Helper Functions
// ===========================================

async function executeWithProgress<T>(
    title: string,
    task: () => Promise<T>
): Promise<T | undefined> {
    return vscode.window.withProgress(
        {
            location: vscode.ProgressLocation.Notification,
            title,
            cancellable: true,
        },
        async (progress, token) => {
            try {
                return await task();
            } catch (error: any) {
                vscode.window.showErrorMessage(`Error: ${error.message}`);
                return undefined;
            }
        }
    );
}

function getLanguageId(editor: vscode.TextEditor): string {
    return editor.document.languageId || 'plaintext';
}

async function showResponsePanel(title: string, content: string, showApply: boolean = false) {
    const panel = vscode.window.createWebviewPanel(
        'llmCopilotResponse',
        title,
        vscode.ViewColumn.Beside,
        { enableScripts: true }
    );

    panel.webview.html = getResponseHtml(title, content, showApply);

    if (showApply) {
        panel.webview.onDidReceiveMessage(async (message) => {
            if (message.command === 'apply') {
                await applyCodeToEditor(message.code);
            } else if (message.command === 'copy') {
                await vscode.env.clipboard.writeText(message.code);
                vscode.window.showInformationMessage('Code copied to clipboard');
            }
        });
    }
}

async function applyCodeToEditor(code: string) {
    const editor = vscode.window.activeTextEditor;
    if (!editor) {
        vscode.window.showWarningMessage('No editor is active');
        return;
    }

    await editor.edit((editBuilder) => {
        editBuilder.replace(editor.selection, code);
    });

    vscode.window.showInformationMessage('Code applied successfully');
}

function getResponseHtml(title: string, content: string, showApply: boolean): string {
    // Extract code blocks
    const codeBlockRegex = /```(\w+)?\n([\s\S]*?)```/g;
    let formattedContent = content;
    const codeBlocks: string[] = [];

    formattedContent = content.replace(codeBlockRegex, (match, lang, code) => {
        const index = codeBlocks.length;
        codeBlocks.push(code.trim());
        return `<div class="code-block">
            <div class="code-header">
                <span>${lang || 'code'}</span>
                ${showApply ? `<button onclick="applyCode(${index})">Apply</button>` : ''}
                <button onclick="copyCode(${index})">Copy</button>
            </div>
            <pre><code>${escapeHtml(code.trim())}</code></pre>
        </div>`;
    });

    return `<!DOCTYPE html>
    <html>
    <head>
        <style>
            body {
                font-family: var(--vscode-font-family);
                color: var(--vscode-foreground);
                background: var(--vscode-editor-background);
                padding: 16px;
                line-height: 1.6;
            }
            h1 { font-size: 1.4em; margin-bottom: 16px; }
            .code-block {
                background: var(--vscode-textBlockQuote-background);
                border-radius: 4px;
                margin: 16px 0;
                overflow: hidden;
            }
            .code-header {
                display: flex;
                justify-content: space-between;
                align-items: center;
                padding: 8px 12px;
                background: var(--vscode-titleBar-activeBackground);
                font-size: 0.9em;
            }
            .code-header button {
                background: var(--vscode-button-background);
                color: var(--vscode-button-foreground);
                border: none;
                padding: 4px 12px;
                border-radius: 4px;
                cursor: pointer;
                margin-left: 8px;
            }
            .code-header button:hover {
                background: var(--vscode-button-hoverBackground);
            }
            pre {
                margin: 0;
                padding: 12px;
                overflow-x: auto;
            }
            code {
                font-family: var(--vscode-editor-font-family);
                font-size: var(--vscode-editor-font-size);
            }
            p { margin: 8px 0; }
        </style>
    </head>
    <body>
        <h1>${escapeHtml(title)}</h1>
        <div>${formattedContent.replace(/\n/g, '<br>')}</div>
        <script>
            const vscode = acquireVsCodeApi();
            const codeBlocks = ${JSON.stringify(codeBlocks)};

            function applyCode(index) {
                vscode.postMessage({ command: 'apply', code: codeBlocks[index] });
            }

            function copyCode(index) {
                vscode.postMessage({ command: 'copy', code: codeBlocks[index] });
            }
        </script>
    </body>
    </html>`;
}

function escapeHtml(text: string): string {
    return text
        .replace(/&/g, '&amp;')
        .replace(/</g, '&lt;')
        .replace(/>/g, '&gt;')
        .replace(/"/g, '&quot;')
        .replace(/'/g, '&#039;');
}
