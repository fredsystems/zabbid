// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

/**
 * Navigation component with dropdown menu for admin navigation.
 * Mobile-first design with clear visual hierarchy.
 */

import { useEffect, useRef, useState } from "react";
import { useLocation, useNavigate } from "react-router-dom";
import type { GlobalCapabilities } from "../types";

interface NavigationProps {
  capabilities: GlobalCapabilities | null;
}

export function Navigation({ capabilities }: NavigationProps) {
  const [isOpen, setIsOpen] = useState(false);
  const navigate = useNavigate();
  const location = useLocation();
  const dropdownRef = useRef<HTMLDivElement>(null);

  // Debug logging
  console.log("[Navigation] Capabilities:", capabilities);

  // Close dropdown when clicking outside
  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (
        dropdownRef.current &&
        !dropdownRef.current.contains(event.target as Node)
      ) {
        setIsOpen(false);
      }
    };

    if (isOpen) {
      document.addEventListener("mousedown", handleClickOutside);
    }

    return () => {
      document.removeEventListener("mousedown", handleClickOutside);
    };
  }, [isOpen]);

  const handleNavigation = (path: string) => {
    navigate(path);
    setIsOpen(false);
  };

  const getCurrentPage = () => {
    if (location.pathname === "/admin") return "Dashboard";
    if (location.pathname.startsWith("/admin/bootstrap"))
      return "Bootstrap Setup";
    if (location.pathname.startsWith("/admin/round-groups"))
      return "Round Groups";
    if (location.pathname.startsWith("/admin/operators"))
      return "Operator Management";
    if (location.pathname.includes("/areas")) return "Area Management";
    if (location.pathname.includes("/users")) return "User Management";
    return "Dashboard";
  };

  return (
    <div className="navigation-dropdown" ref={dropdownRef}>
      <button
        type="button"
        className="nav-toggle"
        onClick={() => setIsOpen(!isOpen)}
        aria-expanded={isOpen}
        aria-label="Navigation menu"
      >
        <span className="nav-current">{getCurrentPage()}</span>
        <span className="nav-arrow">{isOpen ? "▲" : "▼"}</span>
      </button>

      {isOpen && (
        <div className="nav-menu">
          <button
            type="button"
            onClick={() => handleNavigation("/admin")}
            className={location.pathname === "/admin" ? "active" : ""}
          >
            Dashboard
          </button>
          <button
            type="button"
            onClick={() => handleNavigation("/admin/bootstrap")}
            className={
              location.pathname.startsWith("/admin/bootstrap") ? "active" : ""
            }
          >
            Bootstrap Setup
          </button>
          <button
            type="button"
            onClick={() => handleNavigation("/admin/round-groups")}
            className={
              location.pathname.startsWith("/admin/round-groups")
                ? "active"
                : ""
            }
          >
            Round Groups
          </button>
          {capabilities?.can_create_operator && (
            <button
              type="button"
              onClick={() => handleNavigation("/admin/operators")}
              className={
                location.pathname.startsWith("/admin/operators") ? "active" : ""
              }
            >
              Operator Management
            </button>
          )}
        </div>
      )}
    </div>
  );
}
