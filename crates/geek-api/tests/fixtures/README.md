These JSON files are what GeekVPNBot actually sends: they were produced by
serialising its own response models (`AppLinkStartResponse`,
`AppLinkPollResponse`, `AppPasswordResponse`, `TokenResponse`, `UserResponse`
and `problem_response`) with `model_dump(mode="json", by_alias=True)`, exactly
as FastAPI does. When the backend's app API changes, regenerate them from
that repository rather than editing them by hand.
