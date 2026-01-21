-- Phase 29B Semantic Correction: Areas reference round groups, rounds belong to round groups only

-- Step 1: Drop foreign key constraint from rounds.area_id (must be done before dropping index)
ALTER TABLE rounds DROP FOREIGN KEY rounds_ibfk_1;

-- Step 2: Drop unique index on rounds(area_id, round_number)
ALTER TABLE rounds DROP INDEX area_id;

-- Step 3: Drop area_id column from rounds
ALTER TABLE rounds DROP COLUMN area_id;

-- Step 4: Add round_group_id to areas table
ALTER TABLE areas ADD COLUMN round_group_id BIGINT;

-- Step 5: Add foreign key constraint from areas to round_groups
ALTER TABLE areas ADD CONSTRAINT fk_areas_round_group_id
    FOREIGN KEY (round_group_id) REFERENCES round_groups(round_group_id);

-- Step 6: Add new unique constraint on rounds(round_group_id, round_number)
ALTER TABLE rounds ADD UNIQUE INDEX round_group_id (round_group_id, round_number);
