These JSON files are what GeekVPNBot actually sends: they were produced by
serialising its own response models (`AppLinkStartResponse`,
`AppLinkPollResponse`, `AppPasswordResponse`, `TokenResponse`, `UserResponse`
and `problem_response`) with `model_dump(mode="json", by_alias=True)`, exactly
as FastAPI does. When the backend's app API changes, regenerate them from
that repository rather than editing them by hand.

The Mini App fixtures (shop, wallet, trial, referral, usage, tickets) come
from `miniapp_fixtures.py`, which builds the backend's own read models and
serialises them the way `routers/miniapp.py` does (`_camelize` over
`jsonable_encoder`, or `model_dump(by_alias=True)` for routes with a response
model). Run it from GeekVPNBot's checkout with its virtualenv:
`python miniapp_fixtures.py <this directory>`.
