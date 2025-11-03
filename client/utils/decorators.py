from functools import wraps

from disnake import Embed

from core.config import get_config

CONFIG = get_config()


def metadata(**meta_kwargs):
    """
    Decorator to attach metadata to commands for enhanced help display
    """

    def decorator(func):
        if not hasattr(func, "__metadata__"):
            func.__metadata__ = {}
        func.__metadata__.update(meta_kwargs)
        return func

    return decorator


def requires_websocket(func):
    """
    Decorator to attach websocket requirement to commands
    """

    @wraps(func)
    async def wrapper(self, ctx, *args, **kwargs):
        if not ctx.bot.websocket or not ctx.bot.websocket.connected:
            embed = Embed(
                description="❌ Backend connection unavailable. Please try again later!",
                color=CONFIG.color_error,
            )
            await ctx.send(embed=embed)
            return None
        return await func(self, ctx, *args, **kwargs)

    return wrapper


def bot_owner_only(func):
    """
    Decorator to restrict command to bot owner only
    """

    @wraps(func)
    async def wrapper(self, ctx, *args, **kwargs):
        if await ctx.bot.is_owner(ctx.author):
            return await func(self, ctx, *args, **kwargs)
        embed = Embed(
            description="❌ You must be the bot owner to use this command!",
            color=CONFIG.color_error,
        )
        return await ctx.send(embed=embed)

    return wrapper


def server_or_bot_owner_only(func):
    """
    Decorator to restrict command to server and bot owner only
    """

    @wraps(func)
    async def wrapper(self, ctx, *args, **kwargs):
        if await ctx.bot.is_owner(ctx.author) or (
            ctx.guild and ctx.guild.owner_id == ctx.author.id
        ):
            return await func(self, ctx, *args, **kwargs)
        embed = Embed(
            description="❌ You must be the server or bot owner to use this command!",
            color=CONFIG.color_error,
        )
        return await ctx.send(embed=embed)

    return wrapper
