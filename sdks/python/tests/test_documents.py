"""
Tests for the documents client.
"""

import pytest
from datetime import datetime
from unittest.mock import AsyncMock, MagicMock, patch

from llm_copilot.documents import (
    AsyncDocumentsClient,
    DocumentsClient,
    Document,
    DocumentChunk,
    DocumentStatus,
    ChunkingStrategy,
    SearchResult,
    SearchResultItem,
    SearchOptions,
)


class TestDocumentModels:
    """Tests for document models."""

    def test_document_model(self):
        """Test Document model."""
        doc = Document(
            id="doc-123",
            filename="test.pdf",
            content_type="application/pdf",
            size_bytes=1024,
            chunk_count=5,
            status=DocumentStatus.COMPLETED,
            metadata={"author": "Test"},
            created_at=datetime.now(),
            updated_at=datetime.now(),
        )
        assert doc.id == "doc-123"
        assert doc.filename == "test.pdf"
        assert doc.status == DocumentStatus.COMPLETED

    def test_document_chunk_model(self):
        """Test DocumentChunk model."""
        chunk = DocumentChunk(
            id="chunk-1",
            document_id="doc-123",
            content="Test content",
            index=0,
            token_count=10,
            metadata={},
        )
        assert chunk.id == "chunk-1"
        assert chunk.content == "Test content"

    def test_search_result_model(self):
        """Test SearchResult model."""
        result = SearchResult(
            items=[
                SearchResultItem(
                    chunk_id="chunk-1",
                    document_id="doc-123",
                    content="Test content",
                    score=0.95,
                    metadata={},
                )
            ],
            total_matches=1,
            search_time_ms=50.0,
            query="test query",
        )
        assert len(result.items) == 1
        assert result.items[0].score == 0.95

    def test_search_options_defaults(self):
        """Test SearchOptions default values."""
        options = SearchOptions(query="test")
        assert options.limit == 10
        assert options.threshold == 0.0
        assert options.hybrid is False
        assert options.rerank is False


class TestAsyncDocumentsClient:
    """Tests for AsyncDocumentsClient."""

    @pytest.mark.asyncio
    async def test_client_context_manager(self):
        """Test client can be used as context manager."""
        async with AsyncDocumentsClient(base_url="http://localhost:8080") as client:
            assert client._client is not None
        assert client._client is None

    def test_get_headers_with_api_key(self):
        """Test headers include API key."""
        client = AsyncDocumentsClient(
            base_url="http://localhost:8080",
            api_key="test-key",
        )
        headers = client._get_headers()
        assert headers["X-API-Key"] == "test-key"

    @pytest.mark.asyncio
    async def test_get_document(self):
        """Test getting a document."""
        mock_data = {
            "id": "doc-123",
            "filename": "test.pdf",
            "content_type": "application/pdf",
            "size_bytes": 1024,
            "chunk_count": 5,
            "status": "completed",
            "metadata": {},
            "created_at": "2024-01-01T00:00:00Z",
            "updated_at": "2024-01-01T00:00:00Z",
        }

        async with AsyncDocumentsClient(
            base_url="http://localhost:8080",
            api_key="test-key",
        ) as client:
            with patch.object(client, "_request", new_callable=AsyncMock) as mock_request:
                mock_request.return_value = mock_data

                doc = await client.get_document("doc-123")

                assert doc.id == "doc-123"
                assert doc.filename == "test.pdf"
                mock_request.assert_called_once()

    @pytest.mark.asyncio
    async def test_list_documents(self):
        """Test listing documents."""
        mock_data = {
            "items": [
                {
                    "id": "doc-1",
                    "filename": "test1.pdf",
                    "content_type": "application/pdf",
                    "size_bytes": 1024,
                    "chunk_count": 5,
                    "status": "completed",
                    "metadata": {},
                    "created_at": "2024-01-01T00:00:00Z",
                    "updated_at": "2024-01-01T00:00:00Z",
                },
                {
                    "id": "doc-2",
                    "filename": "test2.pdf",
                    "content_type": "application/pdf",
                    "size_bytes": 2048,
                    "chunk_count": 10,
                    "status": "processing",
                    "metadata": {},
                    "created_at": "2024-01-01T00:00:00Z",
                    "updated_at": "2024-01-01T00:00:00Z",
                },
            ]
        }

        async with AsyncDocumentsClient(
            base_url="http://localhost:8080",
            api_key="test-key",
        ) as client:
            with patch.object(client, "_request", new_callable=AsyncMock) as mock_request:
                mock_request.return_value = mock_data

                docs = await client.list_documents()

                assert len(docs) == 2
                assert docs[0].id == "doc-1"
                assert docs[1].id == "doc-2"

    @pytest.mark.asyncio
    async def test_search(self):
        """Test searching documents."""
        mock_data = {
            "items": [
                {
                    "chunk_id": "chunk-1",
                    "document_id": "doc-123",
                    "content": "Test content",
                    "score": 0.95,
                    "metadata": {},
                }
            ],
            "total_matches": 1,
            "search_time_ms": 50.0,
            "query": "test query",
        }

        async with AsyncDocumentsClient(
            base_url="http://localhost:8080",
            api_key="test-key",
        ) as client:
            with patch.object(client, "_request", new_callable=AsyncMock) as mock_request:
                mock_request.return_value = mock_data

                result = await client.search("test query", limit=10)

                assert len(result.items) == 1
                assert result.items[0].score == 0.95
                assert result.query == "test query"

    @pytest.mark.asyncio
    async def test_hybrid_search(self):
        """Test hybrid search."""
        mock_data = {
            "items": [],
            "total_matches": 0,
            "search_time_ms": 25.0,
            "query": "test",
        }

        async with AsyncDocumentsClient(
            base_url="http://localhost:8080",
            api_key="test-key",
        ) as client:
            with patch.object(client, "_request", new_callable=AsyncMock) as mock_request:
                mock_request.return_value = mock_data

                result = await client.hybrid_search(
                    "test",
                    vector_weight=0.6,
                    keyword_weight=0.4,
                )

                call_args = mock_request.call_args
                data = call_args[1]["data"]
                assert data["hybrid"] is True
                assert data["vector_weight"] == 0.6
                assert data["keyword_weight"] == 0.4

    @pytest.mark.asyncio
    async def test_delete_document(self):
        """Test deleting a document."""
        async with AsyncDocumentsClient(
            base_url="http://localhost:8080",
            api_key="test-key",
        ) as client:
            with patch.object(client, "_request", new_callable=AsyncMock) as mock_request:
                mock_request.return_value = {}

                await client.delete_document("doc-123")

                mock_request.assert_called_once_with(
                    "DELETE",
                    "/api/v1/documents/doc-123",
                )


class TestSyncDocumentsClient:
    """Tests for synchronous DocumentsClient."""

    def test_sync_client_delegates_to_async(self):
        """Test sync client properly delegates to async client."""
        client = DocumentsClient(
            base_url="http://localhost:8080",
            api_key="test-key",
        )
        assert client._async_client.api_key == "test-key"
        assert client._async_client.base_url == "http://localhost:8080"
