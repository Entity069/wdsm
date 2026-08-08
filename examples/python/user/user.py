from dataclasses import dataclass


@dataclass
class User:
    id: int
    name: str
    username: str
    age: float
    is_active: bool


def create_user(name: str, email: str, age: float) -> User:
    """Create a new user record from name, email, and age."""
    import random
    return User(
        id=random.randint(1, 100000),
        name=name,
        username=email.split("@")[0],
        age=age,
        is_active=True,
    )
