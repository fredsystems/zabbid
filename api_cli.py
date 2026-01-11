#!/usr/bin/env python3
from __future__ import annotations

import json
import urllib.error
import urllib.parse
import urllib.request
from dataclasses import dataclass
from typing import Literal, Mapping, Optional, Sequence, TypedDict, Union, cast

BASE_URL: str = "http://127.0.0.1:8080"

HttpMethod = Literal["GET", "POST"]

JSONPrimitive = Union[str, int, float, bool, None]
JSONValue = Union[JSONPrimitive, "JSONObject", "JSONArray"]
JSONObject = dict[str, JSONValue]
JSONArray = list[JSONValue]


# -----------------------------
# Session defaults (in-memory)
# -----------------------------
class SessionContext(TypedDict, total=False):
    actor_id: str
    actor_role: str
    cause_id: str
    cause_description: str
    bid_year: int
    area: str


SESSION: SessionContext = {}


# -----------------------------
# Prompt helpers
# -----------------------------
def prompt_str(
    label: str,
    *,
    optional: bool = False,
    default: Optional[str] = None,
) -> str:
    suffix = f" [{default}]" if default is not None else ""
    while True:
        raw: str = input(f"{label}{suffix}: ").strip()
        if raw != "":
            return raw
        if default is not None:
            return default
        if optional:
            return ""
        print("This field is required.")


def prompt_int(label: str, *, default: Optional[int] = None) -> int:
    while True:
        raw: str = prompt_str(
            label, default=str(default) if default is not None else None
        )
        try:
            return int(raw)
        except ValueError:
            print("Please enter a valid integer.")


def prompt_yes_no(label: str, *, default: bool = False) -> bool:
    suffix: str = " [y/N]" if not default else " [Y/n]"
    while True:
        raw: str = input(f"{label}{suffix}: ").strip().lower()
        if raw == "":
            return default
        if raw in ("y", "yes"):
            return True
        if raw in ("n", "no"):
            return False
        print("Please answer y or n.")


# -----------------------------
# Common request fields
# -----------------------------
class ActorEnvelope(TypedDict):
    actor_id: str
    actor_role: str
    cause_id: str
    cause_description: str


def prompt_actor_envelope() -> ActorEnvelope:
    actor_id = prompt_str("Actor ID", default=SESSION.get("actor_id"))
    actor_role = prompt_str(
        "Actor Role (Admin/Bidder)", default=SESSION.get("actor_role")
    )
    cause_id = prompt_str("Cause ID", default=SESSION.get("cause_id"))
    cause_description = prompt_str(
        "Cause description",
        default=SESSION.get("cause_description"),
    )

    SESSION.update(
        actor_id=actor_id,
        actor_role=actor_role,
        cause_id=cause_id,
        cause_description=cause_description,
    )

    return {
        "actor_id": actor_id,
        "actor_role": actor_role,
        "cause_id": cause_id,
        "cause_description": cause_description,
    }


# -----------------------------
# Endpoint definitions
# -----------------------------
@dataclass(frozen=True)
class Endpoint:
    key: str
    name: str
    method: HttpMethod
    path: str


ENDPOINTS: Sequence[Endpoint] = (
    Endpoint("1", "Create Bid Year", "POST", "/bid_years"),
    Endpoint("2", "List Bid Years", "GET", "/bid_years"),
    Endpoint("3", "Create Area", "POST", "/areas"),
    Endpoint("4", "List Areas", "GET", "/areas"),
    Endpoint("5", "Register User", "POST", "/register_user"),
    Endpoint("6", "List Users", "GET", "/users"),
    Endpoint("7", "Leave Availability", "GET", "/leave/availability"),
    Endpoint("8", "Bootstrap Status", "GET", "/bootstrap/status"),
    Endpoint("9", "Checkpoint", "POST", "/checkpoint"),
    Endpoint("10", "Finalize", "POST", "/finalize"),
    Endpoint("11", "Rollback", "POST", "/rollback"),
    Endpoint("12", "Current State", "GET", "/state/current"),
    Endpoint("13", "Historical State", "GET", "/state/historical"),
    Endpoint("14", "Audit Timeline", "GET", "/audit/timeline"),
    # Router: /audit/event/{event_id} — we model this as: base path + prompted event id
    Endpoint("15", "Audit Event by ID", "GET", "/audit/event"),
)


def choose_endpoint() -> Endpoint:
    print("\nAvailable endpoints:")
    for ep in ENDPOINTS:
        print(f"{ep.key}. {ep.name}")

    while True:
        choice: str = prompt_str("Select endpoint")
        for ep in ENDPOINTS:
            if ep.key == choice:
                return ep
        print("Invalid selection.")


# -----------------------------
# Request schemas (TypedDicts)
# -----------------------------
class CreateBidYearRequest(ActorEnvelope):
    year: int
    start_date: str
    num_pay_periods: int


class CreateAreaRequest(ActorEnvelope):
    bid_year: int
    area_id: str


UserType = Literal["CPC", "CPC-IT", "Dev-D", "Dev-R"]


class RegisterUserRequest(ActorEnvelope):
    bid_year: int
    initials: str
    name: str
    area: str
    crew: Optional[int]
    user_type: UserType
    cumulative_natca_bu_date: str
    natca_bu_date: str
    eod_faa_date: str
    service_computation_date: str
    lottery_value: Optional[int]


# -----------------------------
# Build request payloads
# -----------------------------
def build_post_payload(path: str) -> JSONObject:
    """
    Build a JSON body for POST endpoints.
    Returns a JSON object (dict[str, JSONValue]) to keep type checkers happy.
    """
    if path == "/bid_years":
        env: ActorEnvelope = prompt_actor_envelope()
        year: int = prompt_int("Bid year")
        print("Start date must be a Sunday in January (format: YYYY-MM-DD)")
        start_date: str = prompt_str("Start date (YYYY-MM-DD)")
        print("Number of pay periods must be 26 or 27")
        num_pay_periods: int = prompt_int("Number of pay periods (26 or 27)")

        req_year: CreateBidYearRequest = CreateBidYearRequest(
            **env,
            year=year,
            start_date=start_date,
            num_pay_periods=num_pay_periods,
        )
        return cast(JSONObject, req_year)

    if path == "/areas":
        env: ActorEnvelope = prompt_actor_envelope()

        req_area: CreateAreaRequest = {
            "actor_id": env["actor_id"],
            "actor_role": env["actor_role"],
            "cause_id": env["cause_id"],
            "cause_description": env["cause_description"],
            "bid_year": prompt_int("Bid year", default=SESSION.get("bid_year")),
            "area_id": prompt_str("Area ID"),
        }

        SESSION["bid_year"] = req_area["bid_year"]
        SESSION["area"] = req_area["area_id"]

        return cast(JSONObject, req_area)

    if path == "/register_user":
        env = prompt_actor_envelope()
        assign_crew: bool = prompt_yes_no("Assign crew now?", default=False)
        crew_val: Optional[int] = prompt_int("Crew (1-7)") if assign_crew else None

        # keep user_type constrained
        while True:
            ut_raw: str = prompt_str("User type (CPC, CPC-IT, Dev-D, Dev-R)")
            if ut_raw in ("CPC", "CPC-IT", "Dev-D", "Dev-R"):
                user_type_val: UserType = cast(UserType, ut_raw)
                break
            print("Invalid user type. Allowed: CPC, CPC-IT, Dev-D, Dev-R")

        req: RegisterUserRequest = {
            **env,
            "bid_year": prompt_int("Bid year", default=SESSION.get("bid_year")),
            "initials": prompt_str("User initials"),
            "name": prompt_str("User name"),
            "area": prompt_str("Area", default=SESSION.get("area")),
            "crew": crew_val,
            "user_type": user_type_val,
            "cumulative_natca_bu_date": prompt_str(
                "Cumulative NATCA BU date (YYYY-MM-DD)"
            ),
            "natca_bu_date": prompt_str("NATCA BU date (YYYY-MM-DD)"),
            "eod_faa_date": prompt_str("EOD/FAA date (YYYY-MM-DD)"),
            "service_computation_date": prompt_str("SCD (YYYY-MM-DD)"),
            "lottery_value": None,
        }

        SESSION["bid_year"] = req["bid_year"]
        SESSION["area"] = req["area"]

        return cast(JSONObject, req)

    # For now, we don't implement other POST bodies until you provide schemas.
    raise NotImplementedError(f"POST body schema not implemented for {path}")


def build_get_params(path: str) -> Mapping[str, str]:
    """
    Build query params for GET endpoints, returned as strings for urlencode.
    """
    if path == "/bid_years":
        return {}

    if path == "/areas":
        bid_year: int = prompt_int(
            "Bid year",
            default=SESSION.get("bid_year"),
        )
        SESSION["bid_year"] = bid_year
        return {"bid_year": str(bid_year)}

    if path == "/users":
        bid_year: int = prompt_int(
            "Bid year",
            default=SESSION.get("bid_year"),
        )
        area: str = prompt_str(
            "Area",
            default=SESSION.get("area"),
        )
        SESSION.update(bid_year=bid_year, area=area)
        return {"bid_year": str(bid_year), "area": area}

    if path == "/leave/availability":
        bid_year: int = prompt_int(
            "Bid year",
            default=SESSION.get("bid_year"),
        )
        area: str = prompt_str(
            "Area",
            default=SESSION.get("area"),
        )
        initials: str = prompt_str("User initials")
        SESSION.update(bid_year=bid_year, area=area)
        return {"bid_year": str(bid_year), "area": area, "initials": initials}

    if path == "/state/current":
        bid_year: int = prompt_int(
            "Bid year",
            default=SESSION.get("bid_year"),
        )
        area: str = prompt_str(
            "Area",
            default=SESSION.get("area"),
        )
        SESSION.update(bid_year=bid_year, area=area)
        return {"bid_year": str(bid_year), "area": area}

    if path == "/state/historical":
        bid_year: int = prompt_int(
            "Bid year",
            default=SESSION.get("bid_year"),
        )
        area: str = prompt_str(
            "Area",
            default=SESSION.get("area"),
        )
        event_id: int = prompt_int("Event ID")
        SESSION.update(bid_year=bid_year, area=area)
        return {
            "bid_year": str(bid_year),
            "area": area,
            "event_id": str(event_id),
        }

    if path == "/audit/timeline":
        bid_year: int = prompt_int(
            "Bid year",
            default=SESSION.get("bid_year"),
        )
        area: str = prompt_str(
            "Area",
            default=SESSION.get("area"),
        )
        SESSION.update(bid_year=bid_year, area=area)
        return {"bid_year": str(bid_year), "area": area}

    if path == "/audit/event":
        event_id = prompt_int("Event ID")
        return {"__event_id_path__": str(event_id)}

    if path == "/bootstrap/status":
        return {}

    return {}


# -----------------------------
# HTTP functions (stdlib)
# -----------------------------
def http_post(url: str, body: JSONObject) -> tuple[int, str]:
    data: bytes = json.dumps(body).encode("utf-8")
    req = urllib.request.Request(
        url,
        data=data,
        headers={"Content-Type": "application/json"},
        method="POST",
    )
    with urllib.request.urlopen(req) as resp:
        status: int = int(resp.status)
        text: str = resp.read().decode("utf-8")
        return status, text


def http_get(url: str, params: Mapping[str, str]) -> tuple[int, str]:
    full_url: str = url
    if "__event_id_path__" in params:
        event_id: str = params["__event_id_path__"]
        full_url = f"{url}/{urllib.parse.quote(event_id)}"
    else:
        query: str = urllib.parse.urlencode(params)
        if query:
            full_url = f"{url}?{query}"

    with urllib.request.urlopen(full_url) as resp:
        status: int = int(resp.status)
        text: str = resp.read().decode("utf-8")
        return status, text


def print_response(status: int, text: str) -> None:
    print("\nResponse:")
    print(status)
    try:
        parsed = json.loads(text)
        print(json.dumps(parsed, indent=2))
    except json.JSONDecodeError:
        print(text)


# -----------------------------
# Main
# -----------------------------
def main() -> None:
    print("Interactive API client. Ctrl+C or select Quit to exit.")

    while True:
        if SESSION:
            print("\nCurrent session defaults:")
            print(json.dumps(SESSION, indent=2))

        try:
            ep: Endpoint = choose_endpoint()
            print(f"\nSelected: {ep.name}")

            url: str = BASE_URL + ep.path

            if ep.method == "POST":
                payload: JSONObject = build_post_payload(ep.path)
                print("\nRequest JSON:")
                print(json.dumps(payload, indent=2))
                status, text = http_post(url, payload)
                print_response(status, text)
            else:
                params = build_get_params(ep.path)
                if params:
                    print("\nQuery params:")
                    print(json.dumps(dict(params), indent=2))
                status, text = http_get(url, params)
                print_response(status, text)

            print("\n---")

        except KeyboardInterrupt:
            print("\nExiting.")
            return
        except urllib.error.HTTPError as e:
            body = e.read().decode("utf-8")
            print("\nHTTP Error:")
            print(int(e.code))
            print(body)
        except NotImplementedError as e:
            print("\nNot Implemented:")
            print(str(e))


if __name__ == "__main__":
    main()
