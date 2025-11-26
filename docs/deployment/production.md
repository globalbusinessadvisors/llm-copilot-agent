# Production Deployment Guide

This guide covers deploying the LLM CoPilot Agent to a production environment.

## Prerequisites

- Kubernetes cluster (1.28+) or Docker Swarm
- PostgreSQL 16 database
- Redis 7 cluster
- S3-compatible storage (optional)
- Domain name and SSL certificates
- Container registry access

## Architecture Overview

```
                        Internet
                            │
                    ┌───────┴───────┐
                    │  Load Balancer │
                    │   (Ingress)    │
                    └───────┬───────┘
                            │
              ┌─────────────┼─────────────┐
              │             │             │
         ┌────┴────┐  ┌────┴────┐  ┌────┴────┐
         │   API   │  │   API   │  │   API   │
         │ Server  │  │ Server  │  │ Server  │
         │   #1    │  │   #2    │  │   #3    │
         └────┬────┘  └────┬────┘  └────┬────┘
              │             │             │
              └─────────────┴─────────────┘
                            │
           ┌────────────────┼────────────────┐
           │                │                │
      ┌────┴────┐     ┌────┴────┐     ┌────┴────┐
      │ Redis   │     │ Postgres │     │  LLM    │
      │ Cluster │     │ Primary  │     │ Providers│
      └─────────┘     └────┬────┘     └─────────┘
                           │
                      ┌────┴────┐
                      │ Postgres │
                      │ Replica  │
                      └─────────┘
```

## Kubernetes Deployment

### 1. Create Namespace

```yaml
# namespace.yaml
apiVersion: v1
kind: Namespace
metadata:
  name: copilot-prod
  labels:
    name: copilot-prod
```

### 2. Configure Secrets

```yaml
# secrets.yaml
apiVersion: v1
kind: Secret
metadata:
  name: copilot-secrets
  namespace: copilot-prod
type: Opaque
stringData:
  DATABASE_URL: "postgres://user:password@postgres:5432/copilot"
  REDIS_URL: "redis://redis:6379"
  JWT_SECRET: "your-secure-jwt-secret-minimum-32-chars"
  ANTHROPIC_API_KEY: "sk-ant-..."
  OPENAI_API_KEY: "sk-..."
  ENCRYPTION_KEY: "your-32-byte-encryption-key-here"
```

### 3. ConfigMap

```yaml
# configmap.yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: copilot-config
  namespace: copilot-prod
data:
  NODE_ENV: "production"
  LOG_LEVEL: "info"
  LOG_FORMAT: "json"
  PORT: "8080"
  RATE_LIMIT_REQUESTS: "100"
  RATE_LIMIT_WINDOW_MS: "60000"
  MAX_CONVERSATION_MESSAGES: "100"
  DEFAULT_MODEL: "claude-3-sonnet"
```

### 4. API Server Deployment

```yaml
# api-deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: copilot-api
  namespace: copilot-prod
spec:
  replicas: 3
  selector:
    matchLabels:
      app: copilot-api
  template:
    metadata:
      labels:
        app: copilot-api
    spec:
      containers:
      - name: api
        image: llmcopilot/api-server:1.0.0
        ports:
        - containerPort: 8080
        envFrom:
        - configMapRef:
            name: copilot-config
        - secretRef:
            name: copilot-secrets
        resources:
          requests:
            memory: "256Mi"
            cpu: "250m"
          limits:
            memory: "1Gi"
            cpu: "1000m"
        livenessProbe:
          httpGet:
            path: /health
            port: 8080
          initialDelaySeconds: 10
          periodSeconds: 10
        readinessProbe:
          httpGet:
            path: /health
            port: 8080
          initialDelaySeconds: 5
          periodSeconds: 5
        volumeMounts:
        - name: tmp
          mountPath: /tmp
      volumes:
      - name: tmp
        emptyDir: {}
      securityContext:
        runAsNonRoot: true
        runAsUser: 1000
---
apiVersion: v1
kind: Service
metadata:
  name: copilot-api
  namespace: copilot-prod
spec:
  selector:
    app: copilot-api
  ports:
  - port: 80
    targetPort: 8080
  type: ClusterIP
```

### 5. Horizontal Pod Autoscaler

```yaml
# hpa.yaml
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: copilot-api-hpa
  namespace: copilot-prod
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: copilot-api
  minReplicas: 3
  maxReplicas: 10
  metrics:
  - type: Resource
    resource:
      name: cpu
      target:
        type: Utilization
        averageUtilization: 70
  - type: Resource
    resource:
      name: memory
      target:
        type: Utilization
        averageUtilization: 80
```

### 6. Ingress Configuration

```yaml
# ingress.yaml
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: copilot-ingress
  namespace: copilot-prod
  annotations:
    kubernetes.io/ingress.class: nginx
    cert-manager.io/cluster-issuer: letsencrypt-prod
    nginx.ingress.kubernetes.io/proxy-body-size: "50m"
    nginx.ingress.kubernetes.io/proxy-read-timeout: "300"
spec:
  tls:
  - hosts:
    - api.llmcopilot.dev
    secretName: copilot-tls
  rules:
  - host: api.llmcopilot.dev
    http:
      paths:
      - path: /
        pathType: Prefix
        backend:
          service:
            name: copilot-api
            port:
              number: 80
```

### 7. PostgreSQL StatefulSet

```yaml
# postgres.yaml
apiVersion: apps/v1
kind: StatefulSet
metadata:
  name: postgres
  namespace: copilot-prod
spec:
  serviceName: postgres
  replicas: 1
  selector:
    matchLabels:
      app: postgres
  template:
    metadata:
      labels:
        app: postgres
    spec:
      containers:
      - name: postgres
        image: postgres:16
        ports:
        - containerPort: 5432
        env:
        - name: POSTGRES_DB
          value: copilot
        - name: POSTGRES_USER
          valueFrom:
            secretKeyRef:
              name: postgres-secret
              key: username
        - name: POSTGRES_PASSWORD
          valueFrom:
            secretKeyRef:
              name: postgres-secret
              key: password
        - name: PGDATA
          value: /var/lib/postgresql/data/pgdata
        volumeMounts:
        - name: postgres-storage
          mountPath: /var/lib/postgresql/data
        resources:
          requests:
            memory: "1Gi"
            cpu: "500m"
          limits:
            memory: "4Gi"
            cpu: "2000m"
  volumeClaimTemplates:
  - metadata:
      name: postgres-storage
    spec:
      accessModes: ["ReadWriteOnce"]
      storageClassName: fast-ssd
      resources:
        requests:
          storage: 100Gi
```

### 8. Redis Deployment

```yaml
# redis.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: redis
  namespace: copilot-prod
spec:
  replicas: 1
  selector:
    matchLabels:
      app: redis
  template:
    metadata:
      labels:
        app: redis
    spec:
      containers:
      - name: redis
        image: redis:7-alpine
        ports:
        - containerPort: 6379
        command:
        - redis-server
        - --appendonly
        - "yes"
        - --maxmemory
        - "1gb"
        - --maxmemory-policy
        - "allkeys-lru"
        resources:
          requests:
            memory: "512Mi"
            cpu: "250m"
          limits:
            memory: "2Gi"
            cpu: "1000m"
        volumeMounts:
        - name: redis-storage
          mountPath: /data
      volumes:
      - name: redis-storage
        persistentVolumeClaim:
          claimName: redis-pvc
```

## Database Setup

### 1. Initial Migration

```bash
# Run database migrations
kubectl exec -it postgres-0 -n copilot-prod -- psql -U copilot -d copilot -f /migrations/001_initial.sql
```

### 2. Enable pgvector Extension

```sql
-- Enable vector extension
CREATE EXTENSION IF NOT EXISTS vector;

-- Create embedding index
CREATE INDEX ON context_embeddings USING ivfflat (embedding vector_cosine_ops)
    WITH (lists = 100);
```

### 3. Configure Row-Level Security

```sql
-- Enable RLS
ALTER TABLE conversations ENABLE ROW LEVEL SECURITY;
ALTER TABLE messages ENABLE ROW LEVEL SECURITY;

-- Create policies
CREATE POLICY tenant_isolation_conversations ON conversations
    USING (tenant_id = current_setting('app.tenant_id')::uuid);
```

## Environment Variables

### Required Variables

| Variable | Description | Example |
|----------|-------------|---------|
| `DATABASE_URL` | PostgreSQL connection string | `postgres://...` |
| `REDIS_URL` | Redis connection string | `redis://...` |
| `JWT_SECRET` | Secret for JWT signing (min 32 chars) | `...` |
| `ENCRYPTION_KEY` | AES-256 encryption key | `...` |

### LLM Provider Keys

| Variable | Description |
|----------|-------------|
| `ANTHROPIC_API_KEY` | Anthropic API key |
| `OPENAI_API_KEY` | OpenAI API key |
| `GOOGLE_API_KEY` | Google AI API key |

### Optional Configuration

| Variable | Default | Description |
|----------|---------|-------------|
| `PORT` | `8080` | Server port |
| `LOG_LEVEL` | `info` | Logging level |
| `LOG_FORMAT` | `json` | Log format |
| `RATE_LIMIT_REQUESTS` | `100` | Requests per window |
| `RATE_LIMIT_WINDOW_MS` | `60000` | Rate limit window |
| `MAX_RETRIES` | `3` | LLM request retries |
| `TIMEOUT_MS` | `30000` | Request timeout |

## Health Checks

### Liveness Probe

```
GET /health
```

Returns 200 if the service is running.

### Readiness Probe

```
GET /health
```

Returns 200 if the service can accept traffic.

### Startup Probe

```
GET /health
```

Used during container startup.

## Monitoring Setup

### Prometheus Metrics

The API exposes metrics at `/metrics`:

```yaml
# servicemonitor.yaml
apiVersion: monitoring.coreos.com/v1
kind: ServiceMonitor
metadata:
  name: copilot-monitor
  namespace: copilot-prod
spec:
  selector:
    matchLabels:
      app: copilot-api
  endpoints:
  - port: http
    path: /metrics
    interval: 15s
```

### Key Metrics

| Metric | Description |
|--------|-------------|
| `http_requests_total` | Total HTTP requests |
| `http_request_duration_seconds` | Request latency |
| `llm_requests_total` | Total LLM API calls |
| `llm_tokens_used_total` | Total tokens consumed |
| `active_conversations` | Current active conversations |

### Grafana Dashboards

Import the provided dashboards:

- `dashboards/api-overview.json`
- `dashboards/llm-metrics.json`
- `dashboards/error-rates.json`

## Logging

### Log Format

```json
{
  "timestamp": "2024-01-15T12:00:00.000Z",
  "level": "info",
  "service": "api-server",
  "trace_id": "abc123",
  "span_id": "def456",
  "tenant_id": "tenant-789",
  "user_id": "user-012",
  "message": "Request completed",
  "method": "POST",
  "path": "/api/v1/conversations",
  "status": 201,
  "duration_ms": 150
}
```

### Log Aggregation

Configure log shipping to your preferred backend:

```yaml
# fluent-bit configmap
[OUTPUT]
    Name  es
    Match *
    Host  elasticsearch
    Port  9200
    Index copilot-logs
```

## Backup and Recovery

### Database Backups

```bash
# Automated daily backup
kubectl create cronjob pg-backup -n copilot-prod \
  --image=postgres:16 \
  --schedule="0 2 * * *" \
  -- pg_dump -h postgres -U copilot copilot > /backups/copilot-$(date +%Y%m%d).sql
```

### Point-in-Time Recovery

```bash
# Restore from backup
kubectl exec -it postgres-0 -n copilot-prod -- \
  psql -U copilot -d copilot < /backups/copilot-20240115.sql
```

## Security Hardening

### Network Policies

```yaml
# network-policy.yaml
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: api-policy
  namespace: copilot-prod
spec:
  podSelector:
    matchLabels:
      app: copilot-api
  policyTypes:
  - Ingress
  - Egress
  ingress:
  - from:
    - namespaceSelector:
        matchLabels:
          name: ingress-nginx
    ports:
    - protocol: TCP
      port: 8080
  egress:
  - to:
    - podSelector:
        matchLabels:
          app: postgres
    ports:
    - protocol: TCP
      port: 5432
  - to:
    - podSelector:
        matchLabels:
          app: redis
    ports:
    - protocol: TCP
      port: 6379
```

### Pod Security

```yaml
securityContext:
  runAsNonRoot: true
  runAsUser: 1000
  readOnlyRootFilesystem: true
  allowPrivilegeEscalation: false
  capabilities:
    drop:
    - ALL
```

## Scaling Guidelines

### Horizontal Scaling

| Component | Min Replicas | Max Replicas | Scaling Trigger |
|-----------|--------------|--------------|-----------------|
| API Server | 3 | 10 | CPU > 70% |
| Workflow Engine | 2 | 5 | Queue depth > 100 |
| Context Service | 2 | 5 | Memory > 80% |

### Vertical Scaling

| Component | Recommended Resources |
|-----------|----------------------|
| API Server | 1 CPU, 1GB RAM |
| PostgreSQL | 2 CPU, 4GB RAM |
| Redis | 1 CPU, 2GB RAM |

## Troubleshooting

### Common Issues

**1. Database Connection Errors**

```bash
# Check database connectivity
kubectl exec -it copilot-api-xxx -- nc -zv postgres 5432
```

**2. High Memory Usage**

```bash
# Check memory usage
kubectl top pods -n copilot-prod
```

**3. Slow Responses**

```bash
# Check API latency
kubectl logs -l app=copilot-api -n copilot-prod | grep duration_ms
```

## Rollback Procedures

### Kubernetes Rollback

```bash
# Rollback to previous deployment
kubectl rollout undo deployment/copilot-api -n copilot-prod

# Rollback to specific revision
kubectl rollout undo deployment/copilot-api -n copilot-prod --to-revision=2
```

### Database Rollback

```bash
# Rollback migration
kubectl exec -it postgres-0 -n copilot-prod -- \
  psql -U copilot -d copilot -f /migrations/rollback/001_initial.sql
```
