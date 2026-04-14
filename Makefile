.PHONY: run test sample build

run:
	cd backend && uv run uvicorn src.deaths_door.main:app --reload --host 0.0.0.0

test:
	cd backend && uv run pytest

sample:
	cd backend && SAMPLE_GAME=true uv run uvicorn src.deaths_door.main:app --reload --host 0.0.0.0

build:
	cd frontend && npm run build
	rm -rf backend/static/app
	cp -r frontend/dist backend/static/app
