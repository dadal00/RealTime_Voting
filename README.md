Real Time Voting System

![Kapture 2025-03-18 at 16 12 51](https://github.com/user-attachments/assets/8436b200-f370-4ac4-9b18-9f4cab30d57f)

Features
- Concurrent Users
- 4-7ms real time updates

Built with
- Svelte (frontend)
- Go (backend)
- Rust (thread safe storage + counter)
- Kafka (message broker)
  - start topic and send message for updates in rust to kafka
  - if message from kafka, broadcast to all web sockets 

Reflection
- Commit tags
- Possible: Deployment -> mapping Ips and ports using cloudfare tunnel, host local
- Benchmark Process
