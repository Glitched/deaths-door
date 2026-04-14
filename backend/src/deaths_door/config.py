"""Configuration management for Deaths Door application."""

from pathlib import Path


class Config:
    """Application configuration with validation."""

    # Timer Configuration
    TIMER_MAX_SECONDS = 3600  # 1 hour

    # APNS Configuration
    APNS_KEY_PATH = str(Path(__file__).resolve().parent.parent.parent / "keys" / "AuthKey_*.p8")
    APNS_HOST = "https://api.sandbox.push.apple.com"  # Use api.push.apple.com for production
    APNS_TEAM_ID = "WVCM8HLGRN"
    APNS_BUNDLE_ID = "dev.bytealigned.DeathsDoor"
