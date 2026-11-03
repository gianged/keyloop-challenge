# Keyloop Challenge

Hi, Giang here. This is my submission for Scenario A: Unified Service Scheduler. I chose to build the backend only.

- Run everything with one command: `docker compose up --build`. The API listens on `:8080` and Swagger UI is at `/docs`.
- Optional: `cp .env.example .env` to override defaults, or to run outside Docker (`cargo run` needs `DATABASE_URL` exported; the app does not read `.env`).
- All crates are on their latest versions.
- Seeded service types: oil change (30m), brake service (1h), inspection (2h).

# API

| Call | What it does |
|---|---|
| `POST /appointments` | Book an appointment (takes an `Idempotency-Key` header) |
| `GET /appointments/{id}` | Get one appointment |
| `POST /appointments/{id}/cancel` | Cancel an appointment and free its slot |
| `GET /dealerships/{id}/availability` | List open slots by service type and date |
| `GET /dealerships/{id}/events` | SSE stream of booked and cancelled events |
| `GET /healthz` | Health check |

For a ready-made request collection, import `docs/yaak.json`.

# Flow

```
availability -> book -> (events pushed over SSE) -> get / cancel
```

- Booking claims a bay and a qualified technician for the whole service duration.
- No double-booking: Postgres exclusion constraints guarantee it. The in-process lock only makes it faster.

# My story with AI

My story working with AI for now, that I will read the requirement first, then draft the high level design first. After that I have loop that, AI review and suggest, then I make some adjusts and calls.

For the coding stuff, I usually break that into small plan, then have AI doing coding stuff. Then I review the output, see if I like it or understand it or not. (Occasionally, I also write some model properties or function signatures, then have the AI implement. But I rarely doing that nowadays).

# Out of scope

Auth, payment, notifications, technician shifts, opening hours, and scaling reads/writes.
