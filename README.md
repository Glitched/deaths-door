# Death's Door

This repo contains a small tool we wrote to increase the production value when we host games of [Blood on the Clocktower](https://bloodontheclocktower.com/).

The backend component, a [FastAPI](https://fastapi.tiangolo.com/) app, is intended to be run on a computer powering the speakers/music for the event. Currently it has limited soundboard capabilities, but we intend to expand it to include support for setting up [OBS](https://obsproject.com/) scenes with timers, death screens, and more.

The frontend is a [NextJS](https://nextjs.org/) app intended to serve as a small remote, allowing us to trigger sound effects, OBS scenes, or advance the game state.

Currently, we're building out a representation of game state to provide sensible options without large amounts of manual input. The goal is to operate as seamlessly as possible and add to the experience of playing the game rather than detract from it.

## Running the backend

1. Install the font [Help Me](https://www.dafont.com/help-me.font)
2. Install and run [OBS](https://obsproject.com/downloads)
3. Enable the websocket and set the password in your environment as `OBS_PASSWORD`
4. Install packages via [poetry](https://python-poetry.org/docs/) with `poetry install`
5. Entry the poetry virtual env with `poetry shell`
6. Run the backend with `uvicorn src.deaths_door.main:app`. You may add `--reload` to the command to have the server restart when you change the code.

## Frontend

The NextJS frontend has been deprioritized in favor of a native iOS app, which is not yet published.