import pytest

from .helpers import get_test_client


@pytest.mark.anyio
async def test_scripts_list_succeeds():
    """Test that scripts list endpoint works."""
    async with get_test_client() as client:
        response = await client.get("/scripts/list")
    assert response.status_code == 200
    # Just check that we get a response with trouble_brewing
    response_data = response.json()
    assert "trouble_brewing" in response_data
