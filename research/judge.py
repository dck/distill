"""Sonnet 4.6 judge model for DeepEval metrics."""

import json
import os
import re

import anthropic
from deepeval.models import DeepEvalBaseLLM
from pydantic import BaseModel

JUDGE_MODEL = "claude-sonnet-4-6-20250514"


def _make_client() -> anthropic.Anthropic:
    """Create Anthropic client using OAuth token or API key."""
    token = os.environ.get("ANTHROPIC_AUTH_TOKEN")
    if token:
        return anthropic.Anthropic(auth_token=token)
    return anthropic.Anthropic()


class SonnetJudge(DeepEvalBaseLLM):
    def __init__(self):
        super().__init__(model=JUDGE_MODEL)

    def get_model_name(self) -> str:
        return JUDGE_MODEL

    def load_model(self):
        return _make_client()

    def generate(
        self, prompt: str, schema: type[BaseModel] | None = None
    ) -> str | BaseModel:
        if schema is not None:
            prompt += (
                "\n\nRespond with valid JSON matching this schema: "
                f"{json.dumps(schema.model_json_schema())}"
            )

        response = self.model.messages.create(
            model=JUDGE_MODEL,
            max_tokens=4096,
            temperature=0.0,
            messages=[{"role": "user", "content": prompt}],
        )

        text = response.content[0].text

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

        token = os.environ.get("ANTHROPIC_AUTH_TOKEN")
        client = anthropic.AsyncAnthropic(auth_token=token) if token else anthropic.AsyncAnthropic()
        response = await client.messages.create(
            model=JUDGE_MODEL,
            max_tokens=4096,
            temperature=0.0,
            messages=[{"role": "user", "content": prompt}],
        )

        text = response.content[0].text

        if schema is not None:
            return schema.model_validate(json.loads(_extract_json(text)))

        return text


def _extract_json(text: str) -> str:
    """Strip optional markdown code fences around JSON."""
    match = re.search(r"```(?:json)?\s*\n?(.*?)\n?\s*```", text, re.DOTALL)
    return match.group(1).strip() if match else text.strip()
