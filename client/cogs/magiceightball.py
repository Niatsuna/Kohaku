import hashlib
import json
import logging
import re
from datetime import datetime
from random import randrange

from disnake import Embed
from disnake.ext import commands

from core.client import Client
from utils.decorators import metadata

logger = logging.getLogger(__name__)


class MagicEightBallCommand(commands.Cog):
    def __init__(self, client: Client):
        self.client = client

        with open("./data/8ball_answers.json") as f:
            self.answers: list[list[str]] = json.load(f)

    @commands.command(name="8b")
    @commands.cooldown(1, 2, commands.BucketType.user)
    @metadata(
        title="Magic 8-Ball",
        category="Fun",
        usage="<question>",
        description="Ask a yes/no question and receive mystical wisdom!",
    )
    async def magic_eight_ball(self, ctx: commands.Context, *, args: str):
        """
        Answers your questions with yes or no based on the following criteria:
        - case-insensitive message content
        - Author
        - Day
        """
        # Remove special characters, get author's id and current day
        parsed = re.sub(r"[^a-zA-Z0-9]", "", args).lower()
        author_id = ctx.author.id
        day = datetime.today().strftime("%Y-%m-%d")

        # Calculate hash
        message_string = f"{day}.{parsed}.{author_id}"
        hashed_message = int(hashlib.sha1(message_string.encode("utf-8")).hexdigest(), 16)

        # Select answer set (yes / no) based on hash
        answer_set = self.answers[hashed_message % 2]
        answer = f"🎱 {answer_set[randrange(len(answer_set))]}"

        # Build answer embed
        title = (
            args if len(args) <= 256 else f"{args[:250]} [...]"
        )  # Shortens question if question is too long for an embed
        embed = Embed(
            title=title, description=answer, color=self.client.config.color_default
        ).set_author(name=f"{ctx.author.display_name} asked", icon_url=ctx.author.avatar.url)
        await ctx.send(embed=embed)


# ------------------------------------------------------------------------------------------
def setup(client: Client):
    client.add_cog(MagicEightBallCommand(client))
