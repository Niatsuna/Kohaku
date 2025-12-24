import json
from unittest.mock import AsyncMock, MagicMock, mock_open, patch

import disnake
import pytest
from disnake.ext import commands

from cogs.magiceightball import MagicEightBallCommand
from core.client import Client

# ===================== Mocking ===================== #
# Setup mocking parameters like context, client and the setup of the cog


@pytest.fixture
def mock_client():
    """Mock bot client instance"""
    client = MagicMock(spec=Client)
    client.config = MagicMock()
    client.config.color_default = 0x1B6C8E
    return client


@pytest.fixture
def mock_answers():
    """Mock the answers JSON file content"""
    return [["Yes!", "Absolutely", "Positive"], ["No!", "Definitely not", "Ew no"]]


@pytest.fixture
def cog(mock_client, mock_answers):
    """Setups cog with mocked instances"""
    json_data = json.dumps(mock_answers)

    with patch("builtins.open", mock_open(read_data=json_data)):
        return MagicEightBallCommand(mock_client)


@pytest.fixture
def mock_context():
    """Mocks the commands context"""
    ctx = AsyncMock(spec=commands.Context)
    ctx.author = MagicMock()
    ctx.author.id = 12345678
    ctx.author.display_name = "TestUser"
    ctx.author.avatar = MagicMock()
    ctx.author.avatar.url = "https://example.com/avatar.png"
    ctx.send = AsyncMock()
    return ctx


# ===================== Testing ===================== #


@pytest.mark.asyncio
async def test_deterministic_answers(cog, mock_context):
    """Test that same questions in same environment result in the same answer sheet"""
    question = "Is Kohaku a brat?"

    with patch("cogs.magiceightball.datetime") as mock_datetime:
        mock_datetime.today.return_value.strftime.return_value = "2026-01-01"
        await cog.magic_eight_ball.callback(cog, mock_context, args=question)

    # Verify send was called exactly one time
    mock_context.send.assert_called_once()

    # Get answer1
    call_args = mock_context.send.call_args
    embed = call_args.kwargs["embed"]

    assert isinstance(embed, disnake.Embed)
    answer1 = embed.description[2:].strip()
    assert answer1 in (cog.answers[0] + cog.answers[1])

    # Send same question again
    with patch("cogs.magiceightball.datetime") as mock_datetime:
        mock_datetime.today.return_value.strftime.return_value = "2026-01-01"
        await cog.magic_eight_ball.callback(cog, mock_context, args=question)

    # Verify send was called exactly two times now
    assert mock_context.send.call_count == 2

    # Get answer2
    call_args = mock_context.send.call_args
    embed = call_args.kwargs["embed"]

    assert isinstance(embed, disnake.Embed)
    answer2 = embed.description[2:].strip()
    assert answer2 in (cog.answers[0] + cog.answers[1])

    # Verify result (Answers in same answer sheet)
    if answer1 in cog.answers[0]:
        assert answer2 in cog.answers[0]
    else:
        assert answer2 in cog.answers[1]


@pytest.mark.asyncio
async def test_case_insensitive(cog, mock_context):
    """Test that questions are case insensitive"""
    question1 = "Is Kohaku a brat?"
    question2 = "IS KOHAKU A BRAT?"
    question3 = "is kohaku a brat?"

    with patch("cogs.magiceightball.datetime") as mock_datetime:
        mock_datetime.today.return_value.strftime.return_value = "2026-01-01"
        await cog.magic_eight_ball.callback(cog, mock_context, args=question1)

        # Verify call count
        mock_context.send.assert_called_once()

        # Get answer1
        embed = mock_context.send.call_args.kwargs["embed"]
        assert isinstance(embed, disnake.Embed)
        answer1 = embed.description[2:].strip()
        assert answer1 in (cog.answers[0] + cog.answers[1])  # Valid answer

        # Call again (Question 2)
        await cog.magic_eight_ball.callback(cog, mock_context, args=question2)

        # Verify call count
        assert mock_context.send.call_count == 2

        # Get answer2
        embed = mock_context.send.call_args.kwargs["embed"]
        assert isinstance(embed, disnake.Embed)
        answer2 = embed.description[2:].strip()
        assert answer2 in (cog.answers[0] + cog.answers[1])  # Valid answer

        # Call again (Question 3)
        await cog.magic_eight_ball.callback(cog, mock_context, args=question3)

        # Verify call count
        assert mock_context.send.call_count == 3

        # Get answer3
        embed = mock_context.send.call_args.kwargs["embed"]
        assert isinstance(embed, disnake.Embed)
        answer3 = embed.description[2:].strip()
        assert answer3 in (cog.answers[0] + cog.answers[1])  # Valid answer

        # Verify all of them in the same answer sheet
        if answer1 in cog.answers[0]:
            assert answer2 in cog.answers[0]
            assert answer3 in cog.answers[0]
        else:
            assert answer2 in cog.answers[1]
            assert answer3 in cog.answers[1]


@pytest.mark.asyncio
async def test_special_characters(cog, mock_context):
    """Test that special characters have no influence on the corresponding answer"""
    question1 = "Is Kohaku a brat?"
    question2 = "Is@Kohaku!!!a~~brat###?"

    with patch("cogs.magiceightball.datetime") as mock_datetime:
        mock_datetime.today.return_value.strftime.return_value = "2026-01-01"
        await cog.magic_eight_ball.callback(cog, mock_context, args=question1)

        # Verify call count
        mock_context.send.assert_called_once()

        # Get answer1
        embed = mock_context.send.call_args.kwargs["embed"]
        assert isinstance(embed, disnake.Embed)
        answer1 = embed.description[2:].strip()
        assert answer1 in (cog.answers[0] + cog.answers[1])  # Valid answer

        # Call again (Question 2)
        await cog.magic_eight_ball.callback(cog, mock_context, args=question2)

        # Verify call count
        assert mock_context.send.call_count == 2

        # Get answer2
        embed = mock_context.send.call_args.kwargs["embed"]
        assert isinstance(embed, disnake.Embed)
        answer2 = embed.description[2:].strip()
        assert answer2 in (cog.answers[0] + cog.answers[1])  # Valid answer

        # Verify result (Answers in same answer sheet)
        if answer1 in cog.answers[0]:
            assert answer2 in cog.answers[0]
        else:
            assert answer2 in cog.answers[1]
