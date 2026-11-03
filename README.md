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

**Booking** (`POST /appointments`)
1. Validate: `start` is on the slot grid (default 30m) and not in the past. Otherwise 422.
2. Take the per-dealership lock. If it times out, 503 `busy`.
3. Idempotency: if this customer already used the `Idempotency-Key`, return the original booking (200). A different body with the same key gets 422.
4. Check that the service type, customer, vehicle (owned by the customer) and dealership exist. Otherwise 422.
5. Find free bays and free technicians with the required skill, then try inserting each pair until one succeeds.
6. Postgres exclusion constraints on bay, technician and vehicle (confirmed rows only) decide the winner. Nothing free → 409 `slot_unavailable`. Vehicle already booked → 409 `vehicle_unavailable`.
7. New booking: 201 plus a `booked` SSE event.

**Cancel** sets the status to `cancelled`. The constraints only cover confirmed rows, so the slot opens up right away. Calling it twice is safe (200), and it emits a `cancelled` event.

No double-booking: the database guarantees it. The in-process lock only cuts down on wasted inserts.

# My story with AI

My story working with AI for now, that I will read the requirement first, then draft the high level design first. After that I have loop that, AI review and suggest, then I make some adjusts and calls.

For the coding stuff, I usually break that into small plan, then have AI doing coding stuff. Then I review the output, see if I like it or understand it or not. (Occasionally, I also write some model properties or function signatures, then have the AI implement. But I rarely doing that nowadays).

# Out of scope

Auth, payment, notifications, technician shifts, opening hours, and scaling reads/writes.
