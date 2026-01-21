-- Phase 29B Semantic Correction Rollback: Revert to old schema

-- Step 1: Drop the unique constraint on (round_group_id, round_number)
ALTER TABLE rounds DROP INDEX round_group_id;

-- Step 2: Add back area_id column to rounds
ALTER TABLE rounds ADD COLUMN area_id BIGINT NOT NULL;

-- Step 3: Add back foreign key constraint on area_id
ALTER TABLE rounds ADD CONSTRAINT rounds_ibfk_1
    FOREIGN KEY (area_id) REFERENCES areas(area_id);

-- Step 4: Add back unique constraint on (area_id, round_number)
ALTER TABLE rounds ADD UNIQUE INDEX area_id (area_id, round_number);

-- Step 5: Drop foreign key constraint on areas.round_group_id
ALTER TABLE areas DROP FOREIGN KEY fk_areas_round_group_id;

-- Step 6: Drop round_group_id column from areas
ALTER TABLE areas DROP COLUMN round_group_id;
