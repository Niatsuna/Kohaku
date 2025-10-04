from functools import wraps
from disnake.ext import commands

def metadata(**meta_kwargs):
  """
  Decorator to attach metadata to commands for enhanced help display

  Usage:
    @commands.command()
    @metadata(
      category="CategoryA",
      icon="i",
      examples="["-cmd version", "-cmd status"],
      cooldown="5s per user"
    )
    async def cmd(ctx):
      pass
      
  The metadata is stored in command.__metadata__ and can be accessed by the help command for richer display.
  """
  def decorator(func):
    if not hasattr(func, '__metadata__'):
      func.__metadata__ = {}
    func.__metadata__.update(meta_kwargs)
    return func
  return decorator

def requires_websocket(func):
  """
  Decorator to ensure WebSocket is connected before running command.

  Usage:
    @commands.command()
    @requires_websocket
    async def cmd(ctx):
      data = await ctx.bot.websocket.get_data()
  """
  async def wrapper(self, ctx, *args, **kwargs):
    if not ctx.bot.websocket or not ctx.bot.websocket._connected:
      await ctx.send("❌ Backend connection unavailable. Please try again later!")
      return
    return await func(self, ctx, *args, **kwargs)
  return wrapper
