.PHONY: run test sample

run:
	uv run uvicorn src.deaths_door.main:app --reload --host 0.0.0.0

test:
	uv run pytest

sample:
	SAMPLE_GAME=true uv run uvicorn src.deaths_door.main:app --reload --host 0.0.0.0
