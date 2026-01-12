// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

/**
 * CSV User Import component.
 *
 * Implements the multi-step CSV import workflow:
 * 1. Upload CSV content
 * 2. Preview and validate rows
 * 3. Select valid rows to import
 * 4. Confirm import action
 * 5. Execute import
 * 6. Display results
 *
 * This component is strictly non-authoritative:
 * - All validation is performed by the backend
 * - All import logic is owned by the backend
 * - UI only displays what the backend returns
 * - No auto-correction or inference
 */

import { useState } from "react";
import { ApiError, importCsvUsers, previewCsvUsers } from "../api";
import type {
  CsvImportRowResult,
  CsvRowPreview,
  ImportCsvUsersResponse,
  PreviewCsvUsersResponse,
} from "../types";

interface CsvUserImportProps {
  sessionToken: string;
  bidYear: number;
  onImportComplete: () => void;
}

type WorkflowStep = "upload" | "preview" | "confirm" | "importing" | "results";

export function CsvUserImport({
  sessionToken,
  bidYear,
  onImportComplete,
}: CsvUserImportProps) {
  const [step, setStep] = useState<WorkflowStep>("upload");
  const [csvContent, setCsvContent] = useState("");
  const [previewData, setPreviewData] =
    useState<PreviewCsvUsersResponse | null>(null);
  const [selectedRows, setSelectedRows] = useState<Set<number>>(new Set());
  const [importResults, setImportResults] =
    useState<ImportCsvUsersResponse | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  // Step 1: Upload CSV
  const handleCsvUpload = async () => {
    if (!csvContent.trim()) {
      setError("Please paste or upload CSV content");
      return;
    }

    try {
      setLoading(true);
      setError(null);
      const response = await previewCsvUsers(sessionToken, bidYear, csvContent);
      setPreviewData(response);

      // Pre-select all valid rows
      const validIndices = new Set<number>();
      response.rows.forEach((row, index) => {
        if (row.status === "valid") {
          validIndices.add(index);
        }
      });
      setSelectedRows(validIndices);

      setStep("preview");
    } catch (err) {
      if (err instanceof ApiError) {
        setError(`Preview failed: ${err.message}`);
      } else {
        setError(
          "Failed to preview CSV. Please check the format and try again.",
        );
      }
    } finally {
      setLoading(false);
    }
  };

  // Step 2: Toggle row selection
  const toggleRowSelection = (rowIndex: number) => {
    const newSelection = new Set(selectedRows);
    if (newSelection.has(rowIndex)) {
      newSelection.delete(rowIndex);
    } else {
      newSelection.add(rowIndex);
    }
    setSelectedRows(newSelection);
  };

  // Step 3: Proceed to confirmation
  const handleProceedToConfirm = () => {
    if (selectedRows.size === 0) {
      setError("Please select at least one valid row to import");
      return;
    }
    setError(null);
    setStep("confirm");
  };

  // Step 4: Execute import
  const handleConfirmImport = async () => {
    try {
      setLoading(true);
      setError(null);
      setStep("importing");

      const selectedIndices = Array.from(selectedRows).sort((a, b) => a - b);
      const response = await importCsvUsers(
        sessionToken,
        bidYear,
        csvContent,
        selectedIndices,
      );

      // Set results and show them immediately
      setImportResults(response);
      setStep("results");

      // Trigger parent refresh after results are displayed
      // This happens even if there were partial failures in the response
      onImportComplete();
    } catch (err) {
      if (err instanceof ApiError) {
        setError(`Import failed: ${err.message}`);
      } else {
        setError("Import failed. Please try again.");
      }
      setStep("confirm");
    } finally {
      setLoading(false);
    }
  };

  // Reset workflow
  const handleReset = () => {
    setStep("upload");
    setCsvContent("");
    setPreviewData(null);
    setSelectedRows(new Set());
    setImportResults(null);
    setError(null);
  };

  // Render upload step
  const renderUploadStep = () => (
    <div className="csv-import-step upload-step">
      <h3>Upload CSV</h3>
      <p className="step-description">
        Paste or upload CSV content containing user data. The CSV must include a
        header row with required columns.
      </p>

      <div className="csv-textarea-container">
        <label htmlFor="csv-content">CSV Content:</label>
        <textarea
          id="csv-content"
          className="csv-textarea"
          value={csvContent}
          onChange={(e) => setCsvContent(e.target.value)}
          placeholder="Paste CSV content here..."
          rows={15}
          disabled={loading}
        />
      </div>

      {error && <div className="error-message">{error}</div>}

      <div className="step-actions">
        <button
          type="button"
          className="btn-primary"
          onClick={handleCsvUpload}
          disabled={loading || !csvContent.trim()}
        >
          {loading ? "Validating..." : "Preview & Validate"}
        </button>
      </div>
    </div>
  );

  // Render preview step
  const renderPreviewStep = () => {
    if (!previewData) return null;

    return (
      <div className="csv-import-step preview-step">
        <h3>Preview & Select Rows</h3>

        <div className="preview-summary">
          <p>
            <strong>{previewData.total_rows}</strong> rows found:{" "}
            <span className="valid-count">{previewData.valid_count} valid</span>
            ,{" "}
            <span className="invalid-count">
              {previewData.invalid_count} invalid
            </span>
          </p>
          <p>
            <strong>{selectedRows.size}</strong> rows selected for import
          </p>
        </div>

        <div className="preview-rows">
          {previewData.rows.map((row, index) => (
            <PreviewRowCard
              key={`row-${row.row_number}`}
              row={row}
              rowIndex={index}
              isSelected={selectedRows.has(index)}
              onToggle={() => toggleRowSelection(index)}
            />
          ))}
        </div>

        {error && <div className="error-message">{error}</div>}

        <div className="step-actions">
          <button type="button" className="btn-secondary" onClick={handleReset}>
            Cancel
          </button>
          <button
            type="button"
            className="btn-primary"
            onClick={handleProceedToConfirm}
            disabled={selectedRows.size === 0}
          >
            Import Selected ({selectedRows.size})
          </button>
        </div>
      </div>
    );
  };

  // Render confirmation step
  const renderConfirmStep = () => (
    <div className="csv-import-step confirm-step">
      <h3>Confirm Import</h3>

      <div className="confirmation-warning">
        <h4>⚠️ Warning</h4>
        <p>
          You are about to import <strong>{selectedRows.size}</strong> users
          into bid year <strong>{bidYear}</strong>.
        </p>
        <ul>
          <li>
            This action is <strong>irreversible</strong>
          </li>
          <li>One audit event will be created per user</li>
          <li>
            All selected users will be registered in their respective areas
          </li>
        </ul>
      </div>

      {error && <div className="error-message">{error}</div>}

      <div className="step-actions">
        <button
          type="button"
          className="btn-secondary"
          onClick={() => setStep("preview")}
          disabled={loading}
        >
          Back to Preview
        </button>
        <button
          type="button"
          className="btn-danger"
          onClick={handleConfirmImport}
          disabled={loading}
        >
          {loading ? "Importing..." : "Confirm Import"}
        </button>
      </div>
    </div>
  );

  // Render importing step
  const renderImportingStep = () => (
    <div className="csv-import-step importing-step">
      <h3>Importing Users...</h3>
      <p className="loading-message">
        Please wait while users are being imported. Do not close this window.
      </p>
    </div>
  );

  // Render results step
  const renderResultsStep = () => {
    if (!importResults) return null;

    const hasFailures = importResults.failed_count > 0;

    return (
      <div className="csv-import-step results-step">
        <h3>Import Results</h3>

        <div
          className={`results-summary ${hasFailures ? "has-failures" : "all-success"}`}
        >
          <p>
            <strong>{importResults.successful_count}</strong> users imported
            successfully
          </p>
          {hasFailures && (
            <p className="failure-notice">
              <strong>{importResults.failed_count}</strong> users failed to
              import
            </p>
          )}
        </div>

        <div className="results-list">
          {importResults.results.map((result) => (
            <ResultRowCard
              key={`result-${result.row_number}`}
              result={result}
            />
          ))}
        </div>

        <div className="step-actions">
          <button type="button" className="btn-primary" onClick={handleReset}>
            Import More Users
          </button>
        </div>
      </div>
    );
  };

  return (
    <div className="csv-user-import">
      <div className="csv-import-header">
        <h2>CSV User Import</h2>
        <div className="workflow-indicator">
          <span className={step === "upload" ? "active" : ""}>Upload</span>
          <span className="separator">→</span>
          <span className={step === "preview" ? "active" : ""}>Preview</span>
          <span className="separator">→</span>
          <span
            className={
              step === "confirm" || step === "importing" ? "active" : ""
            }
          >
            Confirm
          </span>
          <span className="separator">→</span>
          <span className={step === "results" ? "active" : ""}>Results</span>
        </div>
      </div>

      <div className="csv-import-body">
        {step === "upload" && renderUploadStep()}
        {step === "preview" && renderPreviewStep()}
        {step === "confirm" && renderConfirmStep()}
        {step === "importing" && renderImportingStep()}
        {step === "results" && renderResultsStep()}
      </div>
    </div>
  );
}

// Sub-component for preview row cards
interface PreviewRowCardProps {
  row: CsvRowPreview;
  rowIndex: number;
  isSelected: boolean;
  onToggle: () => void;
}

function PreviewRowCard({ row, isSelected, onToggle }: PreviewRowCardProps) {
  const isValid = row.status === "valid";

  return (
    <div
      className={`preview-row-card ${isValid ? "valid" : "invalid"} ${isSelected ? "selected" : ""}`}
    >
      <div className="row-card-header">
        <div className="row-info">
          <span className="row-number">Row {row.row_number}</span>
          <span className={`status-badge ${row.status}`}>
            {row.status === "valid" ? "✓ Valid" : "✗ Invalid"}
          </span>
        </div>
        {isValid && (
          <label className="row-checkbox">
            <input
              type="checkbox"
              checked={isSelected}
              onChange={onToggle}
              disabled={!isValid}
            />
            <span>Select</span>
          </label>
        )}
      </div>

      <div className="row-card-body">
        <dl className="row-details">
          <dt>Initials:</dt>
          <dd>{row.initials || <em className="missing">—</em>}</dd>

          <dt>Name:</dt>
          <dd>{row.name || <em className="missing">—</em>}</dd>

          <dt>Area:</dt>
          <dd>{row.area_id || <em className="missing">—</em>}</dd>

          <dt>Type:</dt>
          <dd>{row.user_type || <em className="missing">—</em>}</dd>

          <dt>Crew:</dt>
          <dd>
            {row.crew !== null ? row.crew : <em className="missing">None</em>}
          </dd>
        </dl>

        {row.errors.length > 0 && (
          <div className="row-errors">
            <strong>Errors:</strong>
            <ul>
              {row.errors.map((error) => (
                <li key={error}>{error}</li>
              ))}
            </ul>
          </div>
        )}
      </div>
    </div>
  );
}

// Sub-component for result row cards
interface ResultRowCardProps {
  result: CsvImportRowResult;
}

function ResultRowCard({ result }: ResultRowCardProps) {
  const isSuccess = result.status === "success";

  return (
    <div className={`result-row-card ${isSuccess ? "success" : "failed"}`}>
      <div className="result-card-header">
        <span className="row-number">Row {result.row_number}</span>
        <span className={`status-badge ${result.status}`}>
          {isSuccess ? "✓ Success" : "✗ Failed"}
        </span>
      </div>

      <div className="result-card-body">
        <p className="user-initials">
          {result.initials || <em>Unknown initials</em>}
        </p>
        {result.error && (
          <div className="result-error">
            <strong>Error:</strong> {result.error}
          </div>
        )}
      </div>
    </div>
  );
}
