.PHONY: run

run:
	cd backend && uvicorn src.deaths_door.main:app --reload --host 0.0.0.0
