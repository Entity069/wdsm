from dataclasses import dataclass
import random


@dataclass
class User:
    id: float
    name: str
    username: str
    age: float
    is_active: bool
    roles: list[str]


class WitWorld:
    def create_user(self, name: str, email: str, age: float) -> User:
        """Create a new user record from name, email, and age."""
        return User(
            id=float(random.randint(1, 100000)),
            name=name,
            username=email.split("@")[0],
            age=age,
            is_active=True,
            roles=["user"],
        )
