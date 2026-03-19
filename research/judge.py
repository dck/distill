"""Opus 4.6 judge model for DeepEval metrics."""

import json
import re

import anthropic
from deepeval.models import DeepEvalBaseLLM
from pydantic import BaseModel


class OpusJudge(DeepEvalBaseLLM):
    def __init__(self):
        super().__init__(model="claude-opus-4-6")

    def get_model_name(self) -> str:
        return "claude-opus-4-6"

    def load_model(self):
        return anthropic.Anthropic()

    def generate(
        self, prompt: str, schema: type[BaseModel] | None = None
    ) -> str | BaseModel:
        if schema is not None:
            prompt += (
                "\n\nRespond with valid JSON matching this schema: "
                f"{json.dumps(schema.model_json_schema())}"
            )

        response = self.model.messages.create(
            model="claude-opus-4-6",
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

        client = anthropic.AsyncAnthropic()
        response = await client.messages.create(
            model="claude-opus-4-6",
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
