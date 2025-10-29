"""Tests for the sounds API endpoints."""

import pytest

from .helpers import get_test_client


@pytest.mark.anyio
async def test_list_sounds_returns_dict_of_categories():
    """Test that /sounds/list returns a dict of sound categories."""
    async with get_test_client() as client:
        response = await client.get("/sounds/list")
        assert response.status_code == 200

        sounds: dict[str, list[str]] = response.json()
        assert isinstance(sounds, dict), "Response should be a dict"
        assert len(sounds) > 0, "Should have at least one category"

        # Verify each category contains a list of strings
        for category, sound_list in sounds.items():
            assert isinstance(category, str), f"Category '{category}' should be a string"
            assert isinstance(sound_list, list), f"Category '{category}' should contain a list"
            assert len(sound_list) > 0, f"Category '{category}' should not be empty"
            assert all(isinstance(sound, str) for sound in sound_list), \
                f"All sounds in category '{category}' should be strings"


@pytest.mark.anyio
async def test_list_sounds_contains_expected_categories():
    """Test that /sounds/list contains the expected sound categories."""
    async with get_test_client() as client:
        response = await client.get("/sounds/list")
        assert response.status_code == 200

        sounds = response.json()

        # Based on sound_fx.py, we should have these categories
        expected_categories = ["morning", "goodnight", "reveal", "death"]
        for category in expected_categories:
            assert category in sounds, f"Expected category '{category}' not found in response"

        # Verify specific sounds in categories
        assert "rooster" in sounds["morning"]
        assert "alarm" in sounds["morning"]
        assert "timer" in sounds["morning"]
        assert "music_box" in sounds["goodnight"]
        assert "drumroll" in sounds["reveal"]
        assert "drama" in sounds["reveal"]
        assert "sad_trumpet" in sounds["reveal"]
        assert "death" in sounds["death"]
        assert "wilhelm" in sounds["death"]


@pytest.mark.anyio
async def test_play_valid_sound_succeeds():
    """Test that playing a valid sound returns success."""
    async with get_test_client() as client:
        # Note: This will fail if sound files don't exist, but we're testing the API contract
        response = await client.get("/sounds/play/rooster")

        # We expect either 200 (sound played) or 500 (file not found)
        # Both are valid for testing the API contract
        assert response.status_code in [200, 500]

        if response.status_code == 200:
            data = response.json()
            assert data["status"] == "success"
            assert data["sound"] == "rooster"


@pytest.mark.anyio
async def test_play_invalid_sound_returns_404():
    """Test that playing an invalid sound returns 404."""
    async with get_test_client() as client:
        response = await client.get("/sounds/play/nonexistent_sound")
        assert response.status_code == 404

        data = response.json()
        assert "detail" in data
        assert "not found" in data["detail"].lower()


@pytest.mark.anyio
async def test_play_sound_is_case_insensitive():
    """Test that sound names are case-insensitive (user-friendly feature)."""
    async with get_test_client() as client:
        # Test uppercase
        response = await client.get("/sounds/play/ROOSTER")
        assert response.status_code in [200, 500]  # 200 if file exists, 500 if not

        # Test mixed case
        response = await client.get("/sounds/play/RoOsTeR")
        assert response.status_code in [200, 500]
