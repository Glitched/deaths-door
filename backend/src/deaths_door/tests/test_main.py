import pytest
from httpx import AsyncClient

from deaths_door.main import app


@pytest.mark.anyio
async def test_hello_world_succeeds():
    """Asserts that we return a fun message."""
    async with AsyncClient(app=app, base_url="http://test") as ac:
        response = await ac.get("/scripts")
    assert response.status_code == 200
    assert response.json() == [
        "trouble_brewing",
        "sects_and_violets",
        "bad_moon_rising",
    ]
