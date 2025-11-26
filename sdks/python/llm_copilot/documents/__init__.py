"""
Document management module for the LLM CoPilot Agent SDK.
"""

from llm_copilot.documents.client import AsyncDocumentsClient, DocumentsClient
from llm_copilot.documents.models import (
    ChunkingStrategy,
    Document,
    DocumentChunk,
    DocumentCreate,
    DocumentStatus,
    DocumentUpdate,
    SearchOptions,
    SearchResult,
    SearchResultItem,
)

__all__ = [
    "AsyncDocumentsClient",
    "DocumentsClient",
    "Document",
    "DocumentCreate",
    "DocumentUpdate",
    "DocumentChunk",
    "DocumentStatus",
    "ChunkingStrategy",
    "SearchResult",
    "SearchResultItem",
    "SearchOptions",
]
