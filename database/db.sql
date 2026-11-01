CREATE EXTENSION IF NOT EXISTS btree_gist;

CREATE TABLE dealership (
    id   uuid NOT NULL DEFAULT uuidv7(),
    name text NOT NULL,

    CONSTRAINT pk_dealership PRIMARY KEY (id)
);

CREATE TABLE service_type (
    id               uuid NOT NULL DEFAULT uuidv7(),
    code             text NOT NULL,
    name             text NOT NULL,
    duration_minutes integer NOT NULL,
    required_skill   text NOT NULL,

    CONSTRAINT pk_service_type PRIMARY KEY (id),
    CONSTRAINT uq_service_type_code UNIQUE (code),
    CONSTRAINT chk_service_type_duration_positive CHECK (duration_minutes > 0)
);

CREATE TABLE service_bay (
    id            uuid NOT NULL DEFAULT uuidv7(),
    dealership_id uuid NOT NULL,
    name          text NOT NULL,

    CONSTRAINT pk_service_bay PRIMARY KEY (id)
);

CREATE TABLE technician (
    id            uuid NOT NULL DEFAULT uuidv7(),
    dealership_id uuid NOT NULL,
    name          text NOT NULL,
    skills        text[] NOT NULL DEFAULT '{}',

    CONSTRAINT pk_technician PRIMARY KEY (id)
);

CREATE TABLE customer (
    id   uuid NOT NULL DEFAULT uuidv7(),
    name text NOT NULL,

    CONSTRAINT pk_customer PRIMARY KEY (id)
);

CREATE TABLE vehicle (
    id          uuid NOT NULL DEFAULT uuidv7(),
    customer_id uuid NOT NULL,
    plate       text NOT NULL,

    CONSTRAINT pk_vehicle PRIMARY KEY (id)
);

CREATE TABLE appointment (
    id              uuid NOT NULL DEFAULT uuidv7(),
    customer_id     uuid NOT NULL,
    vehicle_id      uuid NOT NULL,
    dealership_id   uuid NOT NULL,
    service_type_id uuid NOT NULL,
    bay_id          uuid NOT NULL,
    technician_id   uuid NOT NULL,
    slot            tstzrange NOT NULL,
    status          text NOT NULL DEFAULT 'confirmed',
    idempotency_key text NOT NULL,
    created_at      timestamptz NOT NULL DEFAULT now(),

    CONSTRAINT pk_appointment PRIMARY KEY (id),
    CONSTRAINT uq_appointment_customer_id_idempotency_key UNIQUE (customer_id, idempotency_key),
    CONSTRAINT chk_appointment_status_valid CHECK (status IN ('confirmed', 'cancelled')),
    CONSTRAINT chk_appointment_slot_bounded
        CHECK (NOT isempty(slot) AND NOT lower_inf(slot) AND NOT upper_inf(slot)),
    CONSTRAINT ex_appointment_bay_id_slot
        EXCLUDE USING gist (bay_id WITH =, slot WITH &&) WHERE (status = 'confirmed'),
    CONSTRAINT ex_appointment_technician_id_slot
        EXCLUDE USING gist (technician_id WITH =, slot WITH &&) WHERE (status = 'confirmed'),
    CONSTRAINT ex_appointment_vehicle_id_slot
        EXCLUDE USING gist (vehicle_id WITH =, slot WITH &&) WHERE (status = 'confirmed')
);

ALTER TABLE service_bay
    ADD CONSTRAINT fk_service_bay_dealership_id
    FOREIGN KEY (dealership_id) REFERENCES dealership (id)
    ON DELETE RESTRICT;

ALTER TABLE technician
    ADD CONSTRAINT fk_technician_dealership_id
    FOREIGN KEY (dealership_id) REFERENCES dealership (id)
    ON DELETE RESTRICT;

ALTER TABLE vehicle
    ADD CONSTRAINT fk_vehicle_customer_id
    FOREIGN KEY (customer_id) REFERENCES customer (id)
    ON DELETE RESTRICT;

ALTER TABLE appointment
    ADD CONSTRAINT fk_appointment_customer_id
    FOREIGN KEY (customer_id) REFERENCES customer (id)
    ON DELETE RESTRICT;

ALTER TABLE appointment
    ADD CONSTRAINT fk_appointment_vehicle_id
    FOREIGN KEY (vehicle_id) REFERENCES vehicle (id)
    ON DELETE RESTRICT;

ALTER TABLE appointment
    ADD CONSTRAINT fk_appointment_dealership_id
    FOREIGN KEY (dealership_id) REFERENCES dealership (id)
    ON DELETE RESTRICT;

ALTER TABLE appointment
    ADD CONSTRAINT fk_appointment_service_type_id
    FOREIGN KEY (service_type_id) REFERENCES service_type (id)
    ON DELETE RESTRICT;

ALTER TABLE appointment
    ADD CONSTRAINT fk_appointment_bay_id
    FOREIGN KEY (bay_id) REFERENCES service_bay (id)
    ON DELETE RESTRICT;

ALTER TABLE appointment
    ADD CONSTRAINT fk_appointment_technician_id
    FOREIGN KEY (technician_id) REFERENCES technician (id)
    ON DELETE RESTRICT;

CREATE INDEX idx_service_bay_dealership_id ON service_bay (dealership_id);
CREATE INDEX idx_technician_dealership_id ON technician (dealership_id);
CREATE INDEX idx_technician_skills ON technician USING gin (skills);
CREATE INDEX idx_vehicle_customer_id ON vehicle (customer_id);
CREATE INDEX idx_appointment_dealership_id ON appointment (dealership_id);
CREATE INDEX idx_appointment_service_type_id ON appointment (service_type_id);
