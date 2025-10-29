"""Configuration management for Deaths Door application."""

import os


class Config:
    """Application configuration with validation."""

    # OBS Configuration
    OBS_FONT_SIZE = 240
    OBS_DEFAULT_TIMER_SECONDS = 5 * 60

    # Timer Configuration
    TIMER_MAX_SECONDS = 3600  # 1 hour
    TIMER_POLLING_INTERVAL_SECONDS = 0.1

    # Role Reveal Configuration
    ROLE_REVEAL_TIMEOUT_ATTEMPTS = 100

    # Player Configuration
    MAX_PLAYER_NAME_LENGTH = 50

    @staticmethod
    def get_obs_password() -> str:
        """Get OBS password from environment with validation."""
        password = os.getenv("OBS_PASSWORD")
        if not password:
            # Allow development mode with warning
            password = "dev_only"
        return password

    @staticmethod
    def is_obs_required() -> bool:
        """Check if OBS connection is required."""
        return os.getenv("OBS_REQUIRED", "false").lower() == "true"
