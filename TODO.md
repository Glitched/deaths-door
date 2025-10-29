# TODO - Deaths Door Improvements

## Nice-to-Have Features (BOTC Rules)

### Dead Vote Tracking
- **Feature**: Track when dead players have used their single vote
- **Current State**: Player model has `has_used_dead_vote` field and setter, but no UI/API enforcement
- **Implementation**: Add vote counting logic that prevents dead players from voting after they've used their vote
- **Priority**: Medium - improves rules accuracy

### Execution Threshold Calculation
- **Feature**: Calculate and enforce execution threshold (≥50% of living players must vote)
- **Current State**: No automatic calculation or enforcement
- **Implementation**:
  - Add endpoint: `GET /game/execution_threshold` that returns the current threshold
  - Add endpoint: `POST /game/check_execution` that takes vote count and returns if execution succeeds
  - Formula: `threshold = ceil(living_players / 2)`
- **Priority**: Medium - important for accurate game rules

---

## Code Quality Improvements

Based on comprehensive code analysis, here are the most needed improvements prioritized by impact:

## Critical Priority (Security & Stability)

### 1. Fix Blocking Operations in Async Context
- **Location**: `routes/players.py:67-70`
- **Issue**: Polling loop with `sleep(0.1)` blocks the event loop, breaking async performance
- **Fix**: Replace with proper async event-driven approach using asyncio events or WebSocket notifications

### 2. Add Input Validation
- **Issue**: No validation on player names, timer values, or file paths creates security vulnerabilities
- **Locations**: 
  - `routes/players.py:176, 188` - Player names not validated
  - `routes/timer.py:19` - Timer seconds can be negative or extremely large
  - `sound_fx.py:61` - File path construction vulnerable to path injection
- **Fix**: Implement Pydantic validators with proper constraints and sanitization

### 3. Implement Proper Error Handling
- **Locations**:
  - `game.py:64-66, 90-91` - `next()` calls without exception handling
  - `player.py:61` - `remove()` can raise `ValueError`
  - `timer_state.py:18-31` - Too broad exception catches
- **Fix**: Add specific try/catch blocks with proper error responses

### 4. Fix Resource Management
- **Issue**: OBS connections and sound resources never properly cleaned up
- **Locations**: `obs_manager.py`, `sound_fx.py`
- **Fix**: Implement context managers and proper resource cleanup patterns

## High Priority (Architecture & Performance)

### 5. Replace Global State Pattern
- **Location**: `game_manager.py:25`
- **Issue**: Singleton pattern creates tight coupling and makes testing difficult
- **Fix**: Implement dependency injection with proper service interfaces

### 6. Fix Async/Await Issues
- **Location**: `timer_state.py`
- **Issue**: Unawaited coroutines causing runtime warnings
- **Fix**: Proper async task management and cleanup

### 7. Add Authentication
- **Issue**: All API endpoints are completely open with no security
- **Fix**: Implement authentication middleware and rate limiting

### 8. Implement Dependency Injection
- **Issue**: Hard-coded dependencies make code inflexible and untestable
- **Fix**: Use dependency injection framework or manual DI pattern

## Medium Priority (Code Quality)

### 9. Improve Test Coverage
- **Current State**: 73% coverage with all existing tests failing
- **Issues**:
  - Tests try to use uninitialized game state
  - API tests use wrong endpoints (`/script/list` vs `/scripts/list`)
  - No integration tests for critical OBS/sound functionality
- **Fix**: Rewrite failing tests, add comprehensive test suite

### 10. Add Proper Logging
- **Issue**: Currently uses print statements instead of structured logging
- **Fix**: Implement proper logging with different levels and structured output

### 11. Implement Request/Response Models
- **Issue**: API endpoints lack proper Pydantic validation models
- **Fix**: Create comprehensive Pydantic models for all API endpoints

### 12. Reduce Code Duplication
- **Locations**: 
  - Player lookup patterns repeated across routes
  - Similar HTTPException patterns throughout route handlers
  - Status effect validation repeated in multiple places
- **Fix**: Extract common patterns into reusable functions/decorators

## Specific Technical Fixes

### Immediate Code Fixes

```python
# Fix StopIteration handling in game.py:64-66
try:
    character = next(char for char in self.included_roles if char.is_named(role_name))
except StopIteration:
    raise ValueError(f"Role not found in included roles: {role_name}")

# Replace polling in players.py:67-70 with async events
async def wait_for_role_reveal(game: Game, timeout: float = 10.0):
    start_time = asyncio.get_event_loop().time()
    while not game.get_should_reveal_roles():
        if asyncio.get_event_loop().time() - start_time > timeout:
            raise HTTPException(status_code=408, detail="Timeout waiting for role reveal")
        await asyncio.sleep(0.1)
```

### Test Suite Fixes
- Fix test initialization: `Game` needs proper script loading before adding players
- Correct API endpoint URLs in integration tests
- Add proper mocking for OBS and sound dependencies
- Implement test fixtures for consistent game state

### Performance Issues to Address
- Linear searches through player lists - implement indexed lookups
- Sound files loaded on each play - implement pre-loading and caching
- Timer using 1-second sleep intervals - use more efficient timing
- No caching of game state or API responses - add appropriate caching layers

### Security Vulnerabilities
- File path injection in sound loading
- No request size limits
- Weak default OBS password
- No sanitization of user inputs

## Architecture Improvements

1. **Implement Repository Pattern** for game persistence
2. **Add Service Layer** to separate business logic from API routes  
3. **Use Factory Pattern** for character creation
4. **Implement Observer Pattern** for game state changes
5. **Add Configuration Management** for environment-specific settings
6. **Implement Circuit Breaker** for OBS integration
7. **Add Retry Logic** for external service calls
8. **Implement Proper Authentication** and authorization middleware

## Notes

The codebase has solid foundational structure but needs significant improvements in error handling, security, performance, and architectural patterns to be production-ready. The immediate focus should be fixing the blocking operations and test failures to establish a stable foundation for further improvements.