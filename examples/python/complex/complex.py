from dataclasses import dataclass
from typing import Optional


@dataclass
class UserProfile:
    id: float
    username: str
    is_active: bool
    tags: Optional[list[str]]


class WitWorld:
    def process_users(self, users: list[UserProfile], require_active: bool) -> str:
        """Process a list of user profiles and return a summary string."""
        total = len(users)
        active = [u for u in users if u.is_active]

        if require_active and len(active) == 0:
            return "Error: No active users found"

        processed = [f"{u.username} (ID: {u.id}, Tags: {len(u.tags or [])})" for u in users]
        return (
            f"Successfully processed {total} users. "
            f"Active: {len(active)}. "
            f"Details: {', '.join(processed)}"
        )
