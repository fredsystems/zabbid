// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

/**
 * Bid Schedule Setup component.
 *
 * Seventh step in the bootstrap workflow.
 * Allows admin to declare bid timing and window.
 *
 * Functionality:
 * - Set bid timezone (IANA selector)
 * - Set bid start date (date picker; must be Monday and future at confirmation)
 * - Set daily bid window (wall-clock start/end times)
 * - Set bidders per area per day
 * - Display schedule summary
 * - Edit schedule (pre-Canonicalized only)
 *
 * Completion criteria:
 * - All schedule fields are set
 * - Start date is valid
 */

import { useCallback, useEffect, useState } from "react";
import {
  getBidSchedule,
  getBootstrapCompleteness,
  NetworkError,
  setBidSchedule,
} from "../api";
import type {
  BidScheduleInfo,
  ConnectionState,
  GetBootstrapCompletenessResponse,
  GlobalCapabilities,
  LiveEvent,
} from "../types";
import { BootstrapNavigation } from "./BootstrapNavigation";
import { ReadinessWidget } from "./ReadinessWidget";

interface BidScheduleSetupProps {
  sessionToken: string | null;
  capabilities: GlobalCapabilities | null;
  connectionState: ConnectionState;
  lastEvent: LiveEvent | null;
}

export function BidScheduleSetup({
  sessionToken,
  capabilities,
  connectionState,
  lastEvent,
}: BidScheduleSetupProps) {
  const [completeness, setCompleteness] =
    useState<GetBootstrapCompletenessResponse | null>(null);
  const [schedule, setSchedule] = useState<BidScheduleInfo | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const isAdmin = capabilities?.can_create_bid_year ?? false;

  const loadData = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);
      const response = await getBootstrapCompleteness();
      setCompleteness(response);

      if (response.active_bid_year !== null && sessionToken) {
        const activeBidYearInfo = response.bid_years.find(
          (by) => by.year === response.active_bid_year,
        );
        if (activeBidYearInfo) {
          const scheduleResponse = await getBidSchedule(
            sessionToken,
            activeBidYearInfo.bid_year_id,
          );
          setSchedule(scheduleResponse.bid_schedule);
        }
      }
    } catch (err) {
      if (err instanceof NetworkError) {
        setError(
          "Backend is unavailable. Please ensure the server is running.",
        );
      } else {
        setError(
          err instanceof Error
            ? err.message
            : "Failed to load bid schedule setup data",
        );
      }
    } finally {
      setLoading(false);
    }
  }, [sessionToken]);

  useEffect(() => {
    void loadData();
  }, [loadData]);

  useEffect(() => {
    if (connectionState === "connected") {
      void loadData();
    }
  }, [connectionState, loadData]);

  useEffect(() => {
    if (!lastEvent) return;

    if (
      lastEvent.type === "bid_schedule_set" ||
      lastEvent.type === "bid_year_created" ||
      lastEvent.type === "bid_year_updated"
    ) {
      void loadData();
    }
  }, [lastEvent, loadData]);

  if (loading) {
    return <div className="loading">Loading bid schedule setup...</div>;
  }

  if (error) {
    return (
      <div className="error">
        <h2>Unable to Load Bid Schedule Setup</h2>
        <p>{error}</p>
      </div>
    );
  }

  if (!completeness) {
    return <div className="error">No completeness data available</div>;
  }

  if (completeness.active_bid_year === null) {
    return (
      <div className="bootstrap-completeness">
        <BootstrapNavigation currentStep="schedule" />
        <ReadinessWidget
          lifecycleState={completeness.bid_years[0]?.lifecycle_state ?? "Draft"}
          isReadyForBidding={completeness.is_ready_for_bidding}
          blockerCount={
            completeness.blocking_reasons.length +
            completeness.bid_years.reduce(
              (sum, by) => sum + by.blocking_reasons.length,
              0,
            ) +
            completeness.areas.reduce(
              (sum, area) => sum + area.blocking_reasons.length,
              0,
            )
          }
        />
        <div className="bootstrap-content">
          <section className="bootstrap-section">
            <h2 className="section-title">Bid Schedule Setup</h2>
            <p className="section-description">
              No active bid year. Please configure a bid year first.
            </p>
          </section>
        </div>
      </div>
    );
  }

  const activeBidYearInfo = completeness.bid_years.find((by) => by.is_active);

  if (!activeBidYearInfo) {
    return <div className="error">Active bid year not found</div>;
  }

  const isCanonicalizedOrLater =
    activeBidYearInfo.lifecycle_state === "Canonicalized" ||
    activeBidYearInfo.lifecycle_state === "Active" ||
    activeBidYearInfo.lifecycle_state === "Closed";

  return (
    <div className="bootstrap-completeness">
      <BootstrapNavigation currentStep="schedule" />
      <ReadinessWidget
        lifecycleState={activeBidYearInfo.lifecycle_state}
        isReadyForBidding={completeness.is_ready_for_bidding}
        blockerCount={
          completeness.blocking_reasons.length +
          completeness.bid_years.reduce(
            (sum, by) => sum + by.blocking_reasons.length,
            0,
          ) +
          completeness.areas.reduce(
            (sum, area) => sum + area.blocking_reasons.length,
            0,
          )
        }
      />

      <div className="bootstrap-content">
        <section className="bootstrap-section">
          <h2 className="section-title">Bid Schedule Setup</h2>
          <p className="section-description">
            Configure the bidding schedule including timezone, start date, and
            daily bidding window.
          </p>

          {isCanonicalizedOrLater && (
            <div className="info-banner">
              <strong>Note:</strong> Bid schedule is locked after
              canonicalization.
            </div>
          )}

          {/* Schedule blockers would be rendered here if defined in BlockingReason type */}
        </section>

        {schedule && !isCanonicalizedOrLater && (
          <section className="bootstrap-section">
            <BidScheduleDisplay
              schedule={schedule}
              bidYearId={activeBidYearInfo.bid_year_id}
              isAdmin={isAdmin}
              sessionToken={sessionToken}
              onRefresh={loadData}
              onError={setError}
            />
          </section>
        )}

        {schedule && isCanonicalizedOrLater && (
          <section className="bootstrap-section">
            <BidScheduleDisplay
              schedule={schedule}
              bidYearId={activeBidYearInfo.bid_year_id}
              isAdmin={false}
              sessionToken={sessionToken}
              onRefresh={loadData}
              onError={setError}
            />
          </section>
        )}

        {!schedule && isAdmin && !isCanonicalizedOrLater && (
          <section className="bootstrap-section">
            <BidScheduleForm
              bidYearId={activeBidYearInfo.bid_year_id}
              sessionToken={sessionToken}
              onSuccess={loadData}
              onError={setError}
            />
          </section>
        )}

        {!schedule && isCanonicalizedOrLater && (
          <section className="bootstrap-section">
            <p className="empty-state">
              Bid schedule was not configured before canonicalization.
            </p>
          </section>
        )}

        {error && (
          <div className="error-banner">
            <strong>Error:</strong> {error}
          </div>
        )}
      </div>
    </div>
  );
}

// ============================================================================
// Bid Schedule Display Component
// ============================================================================

interface BidScheduleDisplayProps {
  schedule: BidScheduleInfo;
  bidYearId: number;
  isAdmin: boolean;
  sessionToken: string | null;
  onRefresh: () => Promise<void>;
  onError: (error: string) => void;
}

function BidScheduleDisplay({
  schedule,
  bidYearId,
  isAdmin,
  sessionToken,
  onRefresh,
  onError,
}: BidScheduleDisplayProps) {
  const [isEditing, setIsEditing] = useState(false);

  if (isEditing) {
    return (
      <BidScheduleForm
        bidYearId={bidYearId}
        sessionToken={sessionToken}
        existingSchedule={schedule}
        onSuccess={async () => {
          setIsEditing(false);
          await onRefresh();
        }}
        onCancel={() => setIsEditing(false)}
        onError={onError}
      />
    );
  }

  return (
    <div className="bid-schedule-display">
      <div className="schedule-header">
        <h3>Current Bid Schedule</h3>
        {isAdmin && (
          <button
            type="button"
            onClick={() => setIsEditing(true)}
            className="btn-edit"
          >
            Edit
          </button>
        )}
      </div>
      <div className="schedule-details">
        <dl>
          <dt>Timezone:</dt>
          <dd>{schedule.timezone}</dd>

          <dt>Start Date:</dt>
          <dd>{schedule.start_date}</dd>

          <dt>Daily Bid Window:</dt>
          <dd>
            {schedule.window_start_time} – {schedule.window_end_time}
          </dd>

          <dt>Bidders per Area per Day:</dt>
          <dd>{schedule.bidders_per_day}</dd>
        </dl>
      </div>
    </div>
  );
}

// ============================================================================
// Bid Schedule Form Component
// ============================================================================

interface BidScheduleFormProps {
  bidYearId: number;
  sessionToken: string | null;
  existingSchedule?: BidScheduleInfo;
  onSuccess: () => void;
  onCancel?: () => void;
  onError: (error: string) => void;
}

function BidScheduleForm({
  bidYearId,
  sessionToken,
  existingSchedule,
  onSuccess,
  onCancel,
  onError,
}: BidScheduleFormProps) {
  const [timezone, setTimezone] = useState(
    existingSchedule?.timezone ?? "America/New_York",
  );
  const [startDate, setStartDate] = useState(
    existingSchedule?.start_date ?? "",
  );
  const [windowStartTime, setWindowStartTime] = useState(
    existingSchedule?.window_start_time ?? "08:00:00",
  );
  const [windowEndTime, setWindowEndTime] = useState(
    existingSchedule?.window_end_time ?? "17:00:00",
  );
  const [biddersPerDay, setBiddersPerDay] = useState(
    existingSchedule?.bidders_per_day?.toString() ?? "5",
  );
  const [saving, setSaving] = useState(false);

  const handleSave = async () => {
    if (!sessionToken) {
      onError("Session token missing");
      return;
    }

    if (!timezone.trim() || !startDate.trim()) {
      onError("Timezone and start date are required");
      return;
    }

    const biddersNum = Number.parseInt(biddersPerDay, 10);
    if (Number.isNaN(biddersNum) || biddersNum <= 0) {
      onError("Bidders per day must be a positive number");
      return;
    }

    try {
      setSaving(true);
      onError("");
      await setBidSchedule(
        sessionToken,
        bidYearId,
        timezone,
        startDate,
        windowStartTime,
        windowEndTime,
        biddersNum,
      );
      onSuccess();
    } catch (err) {
      if (err instanceof Error) {
        onError(`Failed to set bid schedule: ${err.message}`);
      } else {
        onError("Failed to set bid schedule");
      }
    } finally {
      setSaving(false);
    }
  };

  // Common US timezones for convenience
  const commonTimezones = [
    "America/New_York",
    "America/Chicago",
    "America/Denver",
    "America/Los_Angeles",
    "America/Anchorage",
    "Pacific/Honolulu",
  ];

  return (
    <div className="create-form bid-schedule-form">
      <h3>{existingSchedule ? "Edit Bid Schedule" : "Set Bid Schedule"}</h3>

      <div className="form-row">
        <label htmlFor="timezone">Timezone (IANA):</label>
        <select
          id="timezone"
          value={timezone}
          onChange={(e) => setTimezone(e.target.value)}
          disabled={saving}
        >
          {commonTimezones.map((tz) => (
            <option key={tz} value={tz}>
              {tz}
            </option>
          ))}
        </select>
        <p className="field-hint">
          Select the timezone for bid scheduling. All bid times will be
          interpreted in this timezone.
        </p>
      </div>

      <div className="form-row">
        <label htmlFor="start-date">Start Date (must be a Monday):</label>
        <input
          id="start-date"
          type="date"
          value={startDate}
          onChange={(e) => setStartDate(e.target.value)}
          disabled={saving}
        />
        <p className="field-hint">
          The start date must be a Monday and should be in the future when
          confirming ready to bid.
        </p>
      </div>

      <div className="form-row">
        <label htmlFor="window-start">Daily Bid Window Start Time:</label>
        <input
          id="window-start"
          type="time"
          step="1"
          value={windowStartTime}
          onChange={(e) => setWindowStartTime(`${e.target.value}:00`)}
          disabled={saving}
        />
      </div>

      <div className="form-row">
        <label htmlFor="window-end">Daily Bid Window End Time:</label>
        <input
          id="window-end"
          type="time"
          step="1"
          value={windowEndTime}
          onChange={(e) => setWindowEndTime(`${e.target.value}:00`)}
          disabled={saving}
        />
        <p className="field-hint">
          The daily time window during which bidding is allowed.
        </p>
      </div>

      <div className="form-row">
        <label htmlFor="bidders-per-day">Bidders per Area per Day:</label>
        <input
          id="bidders-per-day"
          type="number"
          min="1"
          value={biddersPerDay}
          onChange={(e) => setBiddersPerDay(e.target.value)}
          disabled={saving}
        />
        <p className="field-hint">
          The maximum number of users allowed to bid per area per day.
        </p>
      </div>

      <div className="form-actions">
        <button
          type="button"
          onClick={handleSave}
          disabled={saving}
          className="btn-save"
        >
          {saving ? "Saving..." : "Save Schedule"}
        </button>
        {onCancel && (
          <button
            type="button"
            onClick={onCancel}
            disabled={saving}
            className="btn-cancel"
          >
            Cancel
          </button>
        )}
      </div>
    </div>
  );
}

// Note: Blocking reason rendering removed - schedule blockers
// would need to be added to the BlockingReason discriminated union type
