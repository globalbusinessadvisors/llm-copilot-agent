"""
Document models for the LLM CoPilot Agent SDK.
"""

from datetime import datetime
from enum import Enum
from typing import Any, Optional

from pydantic import BaseModel, ConfigDict, Field


class DocumentStatus(str, Enum):
    """Status of a document."""

    PENDING = "pending"
    PROCESSING = "processing"
    COMPLETED = "completed"
    FAILED = "failed"


class ChunkingStrategy(str, Enum):
    """Strategy for chunking documents."""

    FIXED_SIZE = "fixed_size"
    SENTENCE = "sentence"
    PARAGRAPH = "paragraph"
    SECTION = "section"
    SEMANTIC = "semantic"
    RECURSIVE = "recursive"


class DocumentCreate(BaseModel):
    """Request to create/upload a document."""

    filename: str
    content_type: Optional[str] = None
    metadata: dict[str, Any] = Field(default_factory=dict)
    chunking_strategy: ChunkingStrategy = ChunkingStrategy.RECURSIVE
    chunk_size: int = 512
    chunk_overlap: int = 50
    generate_embeddings: bool = True
    collection: Optional[str] = None


class DocumentUpdate(BaseModel):
    """Request to update document metadata."""

    metadata: Optional[dict[str, Any]] = None
    collection: Optional[str] = None


class DocumentChunk(BaseModel):
    """A chunk of a document."""

    model_config = ConfigDict(from_attributes=True)

    id: str
    document_id: str
    content: str
    index: int
    token_count: int
    metadata: dict[str, Any] = Field(default_factory=dict)
    embedding_id: Optional[str] = None


class Document(BaseModel):
    """A document in the system."""

    model_config = ConfigDict(from_attributes=True)

    id: str
    filename: str
    content_type: str
    size_bytes: int
    chunk_count: int
    status: DocumentStatus
    error: Optional[str] = None
    metadata: dict[str, Any] = Field(default_factory=dict)
    collection: Optional[str] = None
    created_at: datetime
    updated_at: datetime
    processed_at: Optional[datetime] = None


class SearchResultItem(BaseModel):
    """A single result from a search."""

    chunk_id: str
    document_id: str
    content: str
    score: float
    metadata: dict[str, Any] = Field(default_factory=dict)
    document_filename: Optional[str] = None
    highlight: Optional[str] = None


class SearchResult(BaseModel):
    """Result of a document search."""

    items: list[SearchResultItem]
    total_matches: int
    search_time_ms: float
    query: str


class SearchOptions(BaseModel):
    """Options for document search."""

    query: str
    limit: int = 10
    threshold: float = 0.0
    collection: Optional[str] = None
    hybrid: bool = False
    vector_weight: float = 0.7
    keyword_weight: float = 0.3
    rerank: bool = False
    filters: dict[str, Any] = Field(default_factory=dict)
    include_metadata: bool = True
