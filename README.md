# Death's Door

This repo contains a small tool we wrote to increase the production value when we host games of [Blood on the Clocktower]().

The backend component, a FastAPI app, is intended to be run on a computer powering the speakers/music for the event. Currently it has limited soundboard capabilities, but we intend to expand it to include support for setting up OBS scenes with timers, death screens, and more.

The frontend is a NextJS app intended to serve as a small remote, allowing us to trigger sound effects, OBS scenes, or advance the game state.

Currently, we're building out a representation of game state to provide sensible options without large amounts of manual input. The goal is to operate as seamlessly as possible and add to the experience of playing the game rather than detract from it.