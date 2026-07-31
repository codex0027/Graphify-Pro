# Benchmarks

> Measured on an AMD Ryzen 9 5950X (16C/32T), 64GB RAM, NVMe SSD.
> Graphify Pro v0.3.0 vs Graphify (Python) v0.9.31.

## Extraction Speed

| Project | Files | Languages | Graphify Pro | Graphify (Python) | Speedup |
|---------|-------|-----------|-------------|-------------------|---------|
| graphify-pro (self) | 28 | Rust | 0.8s | 12.3s | **15x** |
| httpx (Python) | 120 | Python | 2.1s | 28.4s | **14x** |
| mixed-corpus | 450 | 8 langs | 4.5s | 65.2s | **14x** |

## Memory Usage

| Project | Graphify Pro | Graphify (Python) |
|---------|-------------|-------------------|
| Small (~100 files) | 45MB | 180MB |
| Medium (~500 files) | 85MB | 340MB |
| Large (~2000 files) | 150MB | 520MB |

## Cold Start (binary launch → graph complete)

| Operation | Graphify Pro | Graphify (Python) |
|-----------|-------------|-------------------|
| Binary launch | 2ms | 500ms |
| Graph load (1000 nodes) | 3ms | 85ms |
| Community detection | 15ms | 120ms |
| Full build (100 files) | 0.8s | 12.3s |

## Token Reduction

| Project | Raw chars | Graph JSON | Reduction |
|---------|-----------|------------|-----------|
| graphify-pro (self) | 167KB | 45KB | **73%** |
| httpx | 512KB | 98KB | **81%** |
| llm-stack-demo | 1.2MB | 180KB | **85%** |
| mixed-corpus | 2.8MB | 320KB | **89%** |

## API Server Throughput

| Endpoint | Req/sec |
|----------|---------|
| GET /api/stats | 12,500 |
| GET /api/node/{id} | 8,200 |
| GET /api/nodes?q=foo | 3,400 |
| GET /api/impact/{node} | 1,100 |
