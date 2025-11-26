"""
Document management client for the LLM CoPilot Agent SDK.
"""

import asyncio
import mimetypes
from pathlib import Path
from typing import Any, BinaryIO, Optional, Union

import httpx

from llm_copilot.documents.models import (
    ChunkingStrategy,
    Document,
    DocumentChunk,
    DocumentCreate,
    DocumentUpdate,
    SearchOptions,
    SearchResult,
)
from llm_copilot.exceptions import CoPilotError, raise_for_status


class AsyncDocumentsClient:
    """
    Async client for document management.

    This client provides methods for uploading, managing, and searching documents.
    Documents are automatically chunked and embedded for semantic search.

    Usage:
        async with AsyncDocumentsClient(base_url, api_key=api_key) as client:
            # Upload a document
            doc = await client.upload_file("path/to/document.pdf")

            # Wait for processing
            doc = await client.wait_for_processing(doc.id)

            # Search documents
            results = await client.search("What is the main topic?")
    """

    def __init__(
        self,
        base_url: str = "http://localhost:8080",
        api_key: Optional[str] = None,
        access_token: Optional[str] = None,
        timeout: float = 60.0,
        verify_ssl: bool = True,
    ):
        """
        Initialize the documents client.

        Args:
            base_url: Base URL of the CoPilot API.
            api_key: API key for authentication.
            access_token: JWT access token (alternative to API key).
            timeout: Request timeout in seconds.
            verify_ssl: Whether to verify SSL certificates.
        """
        self.base_url = base_url.rstrip("/")
        self.api_key = api_key
        self.access_token = access_token
        self.timeout = timeout
        self._verify_ssl = verify_ssl
        self._client: Optional[httpx.AsyncClient] = None

    async def __aenter__(self) -> "AsyncDocumentsClient":
        """Enter async context."""
        self._client = httpx.AsyncClient(
            base_url=self.base_url,
            timeout=self.timeout,
            verify=self._verify_ssl,
        )
        return self

    async def __aexit__(self, exc_type: Any, exc_val: Any, exc_tb: Any) -> None:
        """Exit async context."""
        if self._client:
            await self._client.aclose()
            self._client = None

    @property
    def client(self) -> httpx.AsyncClient:
        """Get the HTTP client."""
        if self._client is None:
            raise RuntimeError("Client not initialized. Use 'async with' context manager.")
        return self._client

    def _get_headers(self) -> dict[str, str]:
        """Get headers for requests."""
        headers: dict[str, str] = {"Accept": "application/json"}
        if self.api_key:
            headers["X-API-Key"] = self.api_key
        elif self.access_token:
            headers["Authorization"] = f"Bearer {self.access_token}"
        return headers

    async def _request(
        self,
        method: str,
        path: str,
        data: Optional[dict[str, Any]] = None,
        params: Optional[dict[str, Any]] = None,
        files: Optional[dict[str, Any]] = None,
    ) -> dict[str, Any]:
        """Make an HTTP request."""
        headers = self._get_headers()

        if files:
            response = await self.client.request(
                method=method,
                url=path,
                data=data,
                params=params,
                files=files,
                headers=headers,
            )
        else:
            headers["Content-Type"] = "application/json"
            response = await self.client.request(
                method=method,
                url=path,
                json=data,
                params=params,
                headers=headers,
            )

        if response.status_code >= 400:
            try:
                error_data = response.json()
            except Exception:
                error_data = {"error": response.text}
            raise_for_status(response.status_code, error_data)

        if response.status_code == 204:
            return {}

        return response.json()

    # ==================== Document Operations ====================

    async def upload_file(
        self,
        file_path: Union[str, Path],
        *,
        metadata: Optional[dict[str, Any]] = None,
        chunking_strategy: ChunkingStrategy = ChunkingStrategy.RECURSIVE,
        chunk_size: int = 512,
        chunk_overlap: int = 50,
        collection: Optional[str] = None,
    ) -> Document:
        """
        Upload a file as a document.

        Args:
            file_path: Path to the file to upload.
            metadata: Optional metadata to attach to the document.
            chunking_strategy: Strategy for chunking the document.
            chunk_size: Target size for chunks (in tokens).
            chunk_overlap: Overlap between chunks (in tokens).
            collection: Optional collection to add the document to.

        Returns:
            The created document.
        """
        path = Path(file_path)
        content_type = mimetypes.guess_type(str(path))[0] or "application/octet-stream"

        with open(path, "rb") as f:
            return await self.upload_bytes(
                f,
                filename=path.name,
                content_type=content_type,
                metadata=metadata,
                chunking_strategy=chunking_strategy,
                chunk_size=chunk_size,
                chunk_overlap=chunk_overlap,
                collection=collection,
            )

    async def upload_bytes(
        self,
        file: BinaryIO,
        filename: str,
        *,
        content_type: str = "application/octet-stream",
        metadata: Optional[dict[str, Any]] = None,
        chunking_strategy: ChunkingStrategy = ChunkingStrategy.RECURSIVE,
        chunk_size: int = 512,
        chunk_overlap: int = 50,
        collection: Optional[str] = None,
    ) -> Document:
        """
        Upload bytes as a document.

        Args:
            file: File-like object with the content.
            filename: Name for the file.
            content_type: MIME type of the content.
            metadata: Optional metadata to attach.
            chunking_strategy: Strategy for chunking.
            chunk_size: Target chunk size.
            chunk_overlap: Overlap between chunks.
            collection: Optional collection name.

        Returns:
            The created document.
        """
        import json

        form_data = {
            "chunking_strategy": chunking_strategy.value,
            "chunk_size": str(chunk_size),
            "chunk_overlap": str(chunk_overlap),
        }
        if metadata:
            form_data["metadata"] = json.dumps(metadata)
        if collection:
            form_data["collection"] = collection

        files = {"file": (filename, file, content_type)}

        data = await self._request(
            "POST",
            "/api/v1/documents",
            data=form_data,
            files=files,
        )
        return Document.model_validate(data)

    async def upload_text(
        self,
        content: str,
        filename: str,
        *,
        metadata: Optional[dict[str, Any]] = None,
        chunking_strategy: ChunkingStrategy = ChunkingStrategy.RECURSIVE,
        chunk_size: int = 512,
        chunk_overlap: int = 50,
        collection: Optional[str] = None,
    ) -> Document:
        """
        Upload text content as a document.

        Args:
            content: Text content to upload.
            filename: Name for the document.
            metadata: Optional metadata.
            chunking_strategy: Chunking strategy.
            chunk_size: Target chunk size.
            chunk_overlap: Overlap between chunks.
            collection: Optional collection name.

        Returns:
            The created document.
        """
        import io

        file = io.BytesIO(content.encode("utf-8"))
        return await self.upload_bytes(
            file,
            filename=filename,
            content_type="text/plain",
            metadata=metadata,
            chunking_strategy=chunking_strategy,
            chunk_size=chunk_size,
            chunk_overlap=chunk_overlap,
            collection=collection,
        )

    async def get_document(self, document_id: str) -> Document:
        """
        Get a document by ID.

        Args:
            document_id: The document ID.

        Returns:
            The document.
        """
        data = await self._request("GET", f"/api/v1/documents/{document_id}")
        return Document.model_validate(data)

    async def list_documents(
        self,
        *,
        collection: Optional[str] = None,
        status: Optional[str] = None,
        limit: int = 20,
        offset: int = 0,
    ) -> list[Document]:
        """
        List documents.

        Args:
            collection: Filter by collection.
            status: Filter by status.
            limit: Maximum number of documents to return.
            offset: Number of documents to skip.

        Returns:
            List of documents.
        """
        params: dict[str, Any] = {"limit": limit, "offset": offset}
        if collection:
            params["collection"] = collection
        if status:
            params["status"] = status

        data = await self._request("GET", "/api/v1/documents", params=params)
        items = data.get("items", data) if isinstance(data, dict) else data
        return [Document.model_validate(doc) for doc in items]

    async def update_document(
        self,
        document_id: str,
        update: DocumentUpdate,
    ) -> Document:
        """
        Update a document's metadata.

        Args:
            document_id: The document ID.
            update: The update to apply.

        Returns:
            The updated document.
        """
        data = await self._request(
            "PATCH",
            f"/api/v1/documents/{document_id}",
            data=update.model_dump(exclude_none=True),
        )
        return Document.model_validate(data)

    async def delete_document(self, document_id: str) -> None:
        """
        Delete a document.

        Args:
            document_id: The document ID.
        """
        await self._request("DELETE", f"/api/v1/documents/{document_id}")

    async def get_document_chunks(
        self,
        document_id: str,
        *,
        limit: int = 50,
        offset: int = 0,
    ) -> list[DocumentChunk]:
        """
        Get chunks for a document.

        Args:
            document_id: The document ID.
            limit: Maximum chunks to return.
            offset: Number of chunks to skip.

        Returns:
            List of document chunks.
        """
        data = await self._request(
            "GET",
            f"/api/v1/documents/{document_id}/chunks",
            params={"limit": limit, "offset": offset},
        )
        items = data.get("items", data) if isinstance(data, dict) else data
        return [DocumentChunk.model_validate(chunk) for chunk in items]

    async def wait_for_processing(
        self,
        document_id: str,
        *,
        poll_interval: float = 1.0,
        timeout: float = 300.0,
    ) -> Document:
        """
        Wait for a document to finish processing.

        Args:
            document_id: The document ID.
            poll_interval: Seconds between status checks.
            timeout: Maximum seconds to wait.

        Returns:
            The processed document.

        Raises:
            CoPilotError: If processing fails or times out.
        """
        import time

        start = time.time()
        while True:
            doc = await self.get_document(document_id)

            if doc.status.value == "completed":
                return doc
            elif doc.status.value == "failed":
                raise CoPilotError(
                    f"Document processing failed: {doc.error}",
                    status_code=500,
                )

            if time.time() - start > timeout:
                raise CoPilotError(
                    f"Timeout waiting for document processing",
                    status_code=408,
                )

            await asyncio.sleep(poll_interval)

    # ==================== Search Operations ====================

    async def search(
        self,
        query: str,
        *,
        limit: int = 10,
        threshold: float = 0.0,
        collection: Optional[str] = None,
        hybrid: bool = False,
        rerank: bool = False,
        filters: Optional[dict[str, Any]] = None,
    ) -> SearchResult:
        """
        Search documents.

        Args:
            query: The search query.
            limit: Maximum results to return.
            threshold: Minimum score threshold.
            collection: Optional collection to search within.
            hybrid: Use hybrid search (vector + keyword).
            rerank: Re-rank results using cross-encoder.
            filters: Optional metadata filters.

        Returns:
            Search results.
        """
        options = SearchOptions(
            query=query,
            limit=limit,
            threshold=threshold,
            collection=collection,
            hybrid=hybrid,
            rerank=rerank,
            filters=filters or {},
        )
        return await self.search_with_options(options)

    async def search_with_options(self, options: SearchOptions) -> SearchResult:
        """
        Search documents with full options.

        Args:
            options: Search options.

        Returns:
            Search results.
        """
        data = await self._request(
            "POST",
            "/api/v1/documents/search",
            data=options.model_dump(),
        )
        return SearchResult.model_validate(data)

    async def hybrid_search(
        self,
        query: str,
        *,
        limit: int = 10,
        vector_weight: float = 0.7,
        keyword_weight: float = 0.3,
        collection: Optional[str] = None,
    ) -> SearchResult:
        """
        Perform hybrid search (vector + keyword).

        Args:
            query: The search query.
            limit: Maximum results.
            vector_weight: Weight for vector similarity.
            keyword_weight: Weight for keyword matching.
            collection: Optional collection filter.

        Returns:
            Search results.
        """
        options = SearchOptions(
            query=query,
            limit=limit,
            hybrid=True,
            vector_weight=vector_weight,
            keyword_weight=keyword_weight,
            collection=collection,
        )
        return await self.search_with_options(options)

    # ==================== Collection Operations ====================

    async def list_collections(self) -> list[str]:
        """
        List all document collections.

        Returns:
            List of collection names.
        """
        data = await self._request("GET", "/api/v1/documents/collections")
        return data.get("collections", [])

    async def delete_collection(self, collection: str) -> None:
        """
        Delete a collection and all its documents.

        Args:
            collection: The collection name.
        """
        await self._request("DELETE", f"/api/v1/documents/collections/{collection}")


class DocumentsClient:
    """
    Synchronous client for document management.

    Usage:
        with DocumentsClient(base_url, api_key=api_key) as client:
            # Upload a document
            doc = client.upload_file("path/to/document.pdf")

            # Wait for processing
            doc = client.wait_for_processing(doc.id)

            # Search documents
            results = client.search("What is the main topic?")
    """

    def __init__(
        self,
        base_url: str = "http://localhost:8080",
        api_key: Optional[str] = None,
        access_token: Optional[str] = None,
        timeout: float = 60.0,
        verify_ssl: bool = True,
    ):
        """Initialize the synchronous documents client."""
        self._async_client = AsyncDocumentsClient(
            base_url=base_url,
            api_key=api_key,
            access_token=access_token,
            timeout=timeout,
            verify_ssl=verify_ssl,
        )
        self._loop: Optional[asyncio.AbstractEventLoop] = None

    def _get_loop(self) -> asyncio.AbstractEventLoop:
        """Get or create an event loop."""
        try:
            return asyncio.get_running_loop()
        except RuntimeError:
            if self._loop is None or self._loop.is_closed():
                self._loop = asyncio.new_event_loop()
            return self._loop

    def _run(self, coro: Any) -> Any:
        """Run a coroutine synchronously."""
        loop = self._get_loop()
        return loop.run_until_complete(coro)

    def __enter__(self) -> "DocumentsClient":
        """Enter context."""
        self._run(self._async_client.__aenter__())
        return self

    def __exit__(self, exc_type: Any, exc_val: Any, exc_tb: Any) -> None:
        """Exit context."""
        self._run(self._async_client.__aexit__(exc_type, exc_val, exc_tb))
        if self._loop and not self._loop.is_running():
            self._loop.close()
            self._loop = None

    def upload_file(
        self,
        file_path: Union[str, Path],
        *,
        metadata: Optional[dict[str, Any]] = None,
        chunking_strategy: ChunkingStrategy = ChunkingStrategy.RECURSIVE,
        chunk_size: int = 512,
        chunk_overlap: int = 50,
        collection: Optional[str] = None,
    ) -> Document:
        """Upload a file as a document."""
        return self._run(
            self._async_client.upload_file(
                file_path,
                metadata=metadata,
                chunking_strategy=chunking_strategy,
                chunk_size=chunk_size,
                chunk_overlap=chunk_overlap,
                collection=collection,
            )
        )

    def upload_text(
        self,
        content: str,
        filename: str,
        *,
        metadata: Optional[dict[str, Any]] = None,
        chunking_strategy: ChunkingStrategy = ChunkingStrategy.RECURSIVE,
        chunk_size: int = 512,
        chunk_overlap: int = 50,
        collection: Optional[str] = None,
    ) -> Document:
        """Upload text content as a document."""
        return self._run(
            self._async_client.upload_text(
                content,
                filename,
                metadata=metadata,
                chunking_strategy=chunking_strategy,
                chunk_size=chunk_size,
                chunk_overlap=chunk_overlap,
                collection=collection,
            )
        )

    def get_document(self, document_id: str) -> Document:
        """Get a document by ID."""
        return self._run(self._async_client.get_document(document_id))

    def list_documents(
        self,
        *,
        collection: Optional[str] = None,
        status: Optional[str] = None,
        limit: int = 20,
        offset: int = 0,
    ) -> list[Document]:
        """List documents."""
        return self._run(
            self._async_client.list_documents(
                collection=collection,
                status=status,
                limit=limit,
                offset=offset,
            )
        )

    def update_document(self, document_id: str, update: DocumentUpdate) -> Document:
        """Update a document's metadata."""
        return self._run(self._async_client.update_document(document_id, update))

    def delete_document(self, document_id: str) -> None:
        """Delete a document."""
        return self._run(self._async_client.delete_document(document_id))

    def get_document_chunks(
        self,
        document_id: str,
        *,
        limit: int = 50,
        offset: int = 0,
    ) -> list[DocumentChunk]:
        """Get chunks for a document."""
        return self._run(
            self._async_client.get_document_chunks(document_id, limit=limit, offset=offset)
        )

    def wait_for_processing(
        self,
        document_id: str,
        *,
        poll_interval: float = 1.0,
        timeout: float = 300.0,
    ) -> Document:
        """Wait for a document to finish processing."""
        return self._run(
            self._async_client.wait_for_processing(
                document_id,
                poll_interval=poll_interval,
                timeout=timeout,
            )
        )

    def search(
        self,
        query: str,
        *,
        limit: int = 10,
        threshold: float = 0.0,
        collection: Optional[str] = None,
        hybrid: bool = False,
        rerank: bool = False,
        filters: Optional[dict[str, Any]] = None,
    ) -> SearchResult:
        """Search documents."""
        return self._run(
            self._async_client.search(
                query,
                limit=limit,
                threshold=threshold,
                collection=collection,
                hybrid=hybrid,
                rerank=rerank,
                filters=filters,
            )
        )

    def hybrid_search(
        self,
        query: str,
        *,
        limit: int = 10,
        vector_weight: float = 0.7,
        keyword_weight: float = 0.3,
        collection: Optional[str] = None,
    ) -> SearchResult:
        """Perform hybrid search."""
        return self._run(
            self._async_client.hybrid_search(
                query,
                limit=limit,
                vector_weight=vector_weight,
                keyword_weight=keyword_weight,
                collection=collection,
            )
        )

    def list_collections(self) -> list[str]:
        """List all document collections."""
        return self._run(self._async_client.list_collections())

    def delete_collection(self, collection: str) -> None:
        """Delete a collection."""
        return self._run(self._async_client.delete_collection(collection))
