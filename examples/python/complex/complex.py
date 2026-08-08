from dataclasses import dataclass
from typing import Optional


@dataclass
class GeoLocation:
    lat: float
    lng: float


@dataclass
class Address:
    street: str
    city: str
    geo: GeoLocation


@dataclass
class UserProfile:
    id: float
    username: str
    is_active: bool
    address: Address
    tags: Optional[list[str]]


@dataclass
class ProcessSummary:
    total_users: float
    active_users: float
    primary_city: str
    details: list[str]


class WitWorld:
    def process_users(self, users: list[UserProfile], require_active: bool) -> ProcessSummary:
        """Process nested user profiles and return a nested summary object."""
        total = float(len(users))
        active = [u for u in users if u.is_active]
        primary_city = users[0].address.city if len(users) > 0 else "Unknown"

        details = [
            f"{u.username} (ID: {u.id}, City: {u.address.city}, Lat: {u.address.geo.lat}, Lng: {u.address.geo.lng}, Tags: {len(u.tags or [])})"
            for u in users
        ]

        return ProcessSummary(
            total_users=total,
            active_users=float(len(active)),
            primary_city=primary_city,
            details=details,
        )
