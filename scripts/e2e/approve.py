# Run with the GeekVPNBot virtualenv and its environment (POSTGRES__*, REDIS__*).
"""Plays the bot's half of app sign-in against the real database: the
customer opened t.me/<bot>?start=applogin_<code> and pressed «تأیید و اتصال»."""
import asyncio, sys, uuid
from datetime import UTC, datetime
from geekvpn.infrastructure.config.settings import get_settings
from geekvpn.infrastructure.di.container import build_container
from geekvpn.infrastructure.di.scope import build_scope
from geekvpn.domain.identity.user import User

TELEGRAM_ID = 1011788123

async def main(deep_link: str, approve: bool) -> None:
    code = deep_link.split("start=applogin_", 1)[1]
    container = build_container(get_settings())
    async with container.unit_of_work() as uow:
        scope = build_scope(container, uow.session)
        if await scope.users.get_by_telegram_id(TELEGRAM_ID) is None:
            await scope.users.add(User.register(user_id=uuid.uuid4(), telegram_id=TELEGRAM_ID,
                referral_code="E2E" + uuid.uuid4().hex[:5].upper(), first_name="امیر", username="amir_e2e",
                now=datetime.now(UTC)))
        req = await scope.app_link_login.claim(code, telegram_user_id=TELEGRAM_ID)
        print("claimed", req.id, req.platform, req.device_name)
        status = await scope.app_link_login.decide(req.id, telegram_user_id=TELEGRAM_ID, approve=approve)
        print("decided", status)
        await uow.commit()

asyncio.run(main(sys.argv[1], sys.argv[2] == "approve" if len(sys.argv) > 2 else True))
