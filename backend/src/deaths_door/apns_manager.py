"""
APNS manager for sending Live Activity push updates.

Gracefully degrades when APNS key is not configured — the iOS app
falls back to its local countdown timer in that case.
"""

from __future__ import annotations

import glob
import logging
import time

import httpx
import jwt

from .config import Config

logger = logging.getLogger(__name__)


class APNSManager:
    """Manages Apple Push Notification Service connections for Live Activity updates."""

    def __init__(self) -> None:
        """Initialize the APNS manager, loading credentials if available."""
        self._push_tokens: set[str] = set()
        self._key: str | None = None
        self._key_id: str | None = None
        self._team_id: str | None = None
        self._bundle_id: str | None = None
        self._available = False

        self._load_config()

    def _load_config(self) -> None:
        """Load APNS credentials from config. Logs warning if unavailable."""
        self._team_id = Config.APNS_TEAM_ID
        self._bundle_id = Config.APNS_BUNDLE_ID
        self._key_id, self._key = self._load_key()

        if not self._key:
            logger.warning(
                "APNS not configured, Live Activity push updates disabled. "
                "Missing: APNS key file (backend/keys/AuthKey_*.p8)"
            )
            return

        self._available = True
        logger.info(f"APNS configured: key={self._key_id}, bundle={self._bundle_id}")

    def _load_key(self) -> tuple[str | None, str | None]:
        """Find and load the .p8 key file from the keys directory."""
        key_files = glob.glob(Config.APNS_KEY_PATH)
        if not key_files:
            return None, None

        key_file = key_files[0]
        # Extract key ID from filename: AuthKey_XXXXXXXXXX.p8
        filename = key_file.rsplit("/", 1)[-1]
        key_id = filename.replace("AuthKey_", "").replace(".p8", "")

        try:
            with open(key_file) as f:
                key_content = f.read()
            return key_id, key_content
        except OSError as e:
            logger.warning(f"Failed to read APNS key file {key_file}: {e}")
            return None, None

    @property
    def is_available(self) -> bool:
        """Whether APNS is configured and ready to send pushes."""
        return self._available

    def register_token(self, token: str) -> None:
        """Register a Live Activity push token."""
        self._push_tokens.add(token)
        logger.info(f"Registered APNS push token: {token[:8]}...")

    def _make_jwt(self) -> str:
        """Create a signed JWT for APNS authentication."""
        assert self._key is not None
        assert self._key_id is not None
        assert self._team_id is not None

        now = int(time.time())
        payload = {
            "iss": self._team_id,
            "iat": now,
        }
        headers = {
            "alg": "ES256",
            "kid": self._key_id,
        }
        return jwt.encode(payload, self._key, algorithm="ES256", headers=headers)

    async def send_timer_update(
        self,
        seconds: int,
        is_running: bool,
        players_alive: int = 0,
        total_players: int = 0,
    ) -> None:
        """
        Send a Live Activity update to all registered tokens.

        Silently no-ops if APNS is not configured or no tokens are registered.
        """
        if not self._available:
            return
        if not self._push_tokens:
            logger.info("APNS: no push tokens registered, skipping")
            return

        logger.info(f"APNS: pushing update (alive={players_alive}/{total_players}, timer={seconds}s, running={is_running})")

        assert self._bundle_id is not None

        # Swift Date decodes as timeIntervalSinceReferenceDate (Jan 1, 2001),
        # not Unix epoch (Jan 1, 1970). Offset: 978307200 seconds.
        end_time = (time.time() + seconds) - 978307200
        content_state = {
            "running": is_running,
            "endTime": end_time,
            "playersAlive": players_alive,
            "totalPlayers": total_players,
        }
        apns_payload = {
            "aps": {
                "timestamp": int(time.time()),
                "event": "update",
                "content-state": content_state,
            },
        }

        token_jwt = self._make_jwt()
        topic = f"{self._bundle_id}.push-type.liveactivity"

        stale_tokens: set[str] = set()

        async with httpx.AsyncClient(http2=True) as client:
            for push_token in self._push_tokens.copy():
                try:
                    url = f"{Config.APNS_HOST}/3/device/{push_token}"
                    response = await client.post(
                        url,
                        json=apns_payload,
                        headers={
                            "authorization": f"bearer {token_jwt}",
                            "apns-topic": topic,
                            "apns-push-type": "liveactivity",
                            "apns-priority": "10",
                        },
                    )
                    if response.status_code == 200:
                        logger.info(f"APNS push sent to {push_token[:8]}...")
                    elif response.status_code == 410:
                        # Token is no longer valid
                        stale_tokens.add(push_token)
                        logger.info(f"APNS token expired, removing: {push_token[:8]}...")
                    else:
                        logger.warning(f"APNS push failed ({response.status_code}): {response.text}")
                except httpx.HTTPError as e:
                    logger.warning(f"APNS request failed: {e}")

        self._push_tokens -= stale_tokens
