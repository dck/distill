"""GPT-4.1 judge model for DeepEval metrics (via Copilot proxy)."""

import asyncio
import json
import re

from deepeval.models import DeepEvalBaseLLM
from openai import AsyncOpenAI, OpenAI
from pydantic import BaseModel

JUDGE_MODEL = "gpt-5-mini"
_COPILOT_PROXY = "http://localhost:4141/v1"
_RETRY_ATTEMPTS = 5
_RETRY_BACKOFF = 3
_MAX_TOKENS = 4096


class GPT5Judge(DeepEvalBaseLLM):
    def __init__(self):
        super().__init__(model=JUDGE_MODEL)
        self._async_client: AsyncOpenAI | None = None

    def get_model_name(self) -> str:
        return JUDGE_MODEL

    def load_model(self):
        import httpx
        return OpenAI(
            base_url=_COPILOT_PROXY, api_key="copilot",
            http_client=httpx.Client(trust_env=False),
        )

    def _get_async_client(self) -> AsyncOpenAI:
        if self._async_client is None:
            import httpx
            self._async_client = AsyncOpenAI(
                base_url=_COPILOT_PROXY, api_key="copilot",
                http_client=httpx.AsyncClient(trust_env=False),
            )
        return self._async_client

    def generate(
        self, prompt: str, schema: type[BaseModel] | None = None
    ) -> str | BaseModel:
        if schema is not None:
            prompt += (
                "\n\nRespond with valid JSON matching this schema: "
                f"{json.dumps(schema.model_json_schema())}"
            )

        response = self.model.chat.completions.create(
            model=JUDGE_MODEL,
            max_tokens=_MAX_TOKENS,
            messages=[{"role": "user", "content": prompt}],
        )

        text = response.choices[0].message.content

        if schema is not None:
            return schema.model_validate(json.loads(_extract_json(text)))

        return text

    async def a_generate(
        self, prompt: str, schema: type[BaseModel] | None = None
    ) -> str | BaseModel:
        if schema is not None:
            prompt += (
                "\n\nRespond with valid JSON matching this schema: "
                f"{json.dumps(schema.model_json_schema())}"
            )

        client = self._get_async_client()
        last_err: Exception | None = None
        for attempt in range(_RETRY_ATTEMPTS):
            try:
                response = await client.chat.completions.create(
                    model=JUDGE_MODEL,
                    max_tokens=_MAX_TOKENS,
                    messages=[{"role": "user", "content": prompt}],
                )
                text = response.choices[0].message.content
                if schema is not None:
                    return schema.model_validate(json.loads(_extract_json(text)))
                return text
            except Exception as e:
                status = getattr(e, "status_code", None)
                if status in (401, 403):
                    raise
                last_err = e
                if attempt < _RETRY_ATTEMPTS - 1:
                    wait = _RETRY_BACKOFF ** (attempt + 1)
                    await asyncio.sleep(wait)

        raise last_err  # type: ignore[misc]


def _extract_json(text: str) -> str:
    """Strip optional markdown code fences around JSON."""
    match = re.search(r"```(?:json)?\s*\n?(.*?)\n?\s*```", text, re.DOTALL)
    return match.group(1).strip() if match else text.strip()
