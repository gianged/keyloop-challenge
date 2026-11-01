CREATE EXTENSION IF NOT EXISTS btree_gist;

CREATE TABLE dealership (
    id   uuid PRIMARY KEY DEFAULT uuidv7(),
    name text NOT NULL
);

CREATE TABLE service_type (
    id               uuid PRIMARY KEY DEFAULT uuidv7(),
    code             text NOT NULL UNIQUE,
    name             text NOT NULL,
    duration_minutes integer NOT NULL CHECK (duration_minutes > 0),
    required_skill   text NOT NULL
);

CREATE TABLE service_bay (
    id            uuid PRIMARY KEY DEFAULT uuidv7(),
    dealership_id uuid NOT NULL REFERENCES dealership (id),
    name          text NOT NULL
);

CREATE INDEX service_bay_dealership_idx ON service_bay (dealership_id);

CREATE TABLE technician (
    id            uuid PRIMARY KEY DEFAULT uuidv7(),
    dealership_id uuid NOT NULL REFERENCES dealership (id),
    name          text NOT NULL,
    skills        text[] NOT NULL DEFAULT '{}'
);

CREATE INDEX technician_dealership_idx ON technician (dealership_id);
CREATE INDEX technician_skills_idx ON technician USING gin (skills);

CREATE TABLE customer (
    id   uuid PRIMARY KEY DEFAULT uuidv7(),
    name text NOT NULL
);

CREATE TABLE vehicle (
    id          uuid PRIMARY KEY DEFAULT uuidv7(),
    customer_id uuid NOT NULL REFERENCES customer (id),
    plate       text NOT NULL
);

CREATE INDEX vehicle_customer_idx ON vehicle (customer_id);

CREATE TABLE appointment (
    id              uuid PRIMARY KEY DEFAULT uuidv7(),
    customer_id     uuid NOT NULL REFERENCES customer (id),
    vehicle_id      uuid NOT NULL REFERENCES vehicle (id),
    dealership_id   uuid NOT NULL REFERENCES dealership (id),
    service_type_id uuid NOT NULL REFERENCES service_type (id),
    bay_id          uuid NOT NULL REFERENCES service_bay (id),
    technician_id   uuid NOT NULL REFERENCES technician (id),
    slot            tstzrange NOT NULL,
    status          text NOT NULL DEFAULT 'confirmed',
    idempotency_key text NOT NULL,
    created_at      timestamptz NOT NULL DEFAULT now(),

    CONSTRAINT appointment_status_valid
        CHECK (status IN ('confirmed', 'cancelled')),
    CONSTRAINT appointment_slot_valid
        CHECK (NOT isempty(slot) AND NOT lower_inf(slot) AND NOT upper_inf(slot)),
    CONSTRAINT appointment_idempotency_key_unique
        UNIQUE (customer_id, idempotency_key),
    CONSTRAINT appointment_bay_no_overlap
        EXCLUDE USING gist (bay_id WITH =, slot WITH &&) WHERE (status = 'confirmed'),
    CONSTRAINT appointment_technician_no_overlap
        EXCLUDE USING gist (technician_id WITH =, slot WITH &&) WHERE (status = 'confirmed'),
    CONSTRAINT appointment_vehicle_no_overlap
        EXCLUDE USING gist (vehicle_id WITH =, slot WITH &&) WHERE (status = 'confirmed')
);
