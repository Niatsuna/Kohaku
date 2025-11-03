from disnake import ButtonStyle, Embed
from disnake.ext import commands
from disnake.ui import Button, View

from core.config import get_config


class KohakuHelpCommand(commands.HelpCommand):
    """
    Custom help command that provides richt, organized help information

    Automatically handles:
    - `-help` - Shows all top-level commands/groups
    - `-help <command/group>` - Shows help for a specific command/group
    """

    def __init__(self):
        config = get_config()
        self.prefix = config.prefix
        self.repo = config.repo
        self.color = config.color_default
        self.color_error = config.color_error

        self.CATEGORY_EMOJIS = {
            "Admin": "🔧",
            "Games": "🎮",
            "Utils": "🔨",
            "Fun": "🎉",
            "Info": "ℹ️",
            "Other": "📦",
        }

        super().__init__(
            command_attrs={
                "aliases": ["h"],
            }
        )

    def get_decorator_var(self, command, key, default=None):
        try:
            return command.callback.__metadata__[key]
        except AttributeError:
            return default

    def get_command_category(self, command):
        """Get the category of a command. If not present, default to category 'Other'"""
        if command.parent is None:
            return self.get_decorator_var(command, "category", default="Other")
        category = self.get_decorator_var(command, "category")
        if category is None:
            return self.get_command_category(command.parent)
        return category

    def get_category_emoji(self, category_name):
        """Get emoji for a category. If not present, default to emoji of 'Other'"""
        if category_name in self.CATEGORY_EMOJIS:
            return self.CATEGORY_EMOJIS[category_name]
        return self.CATEGORY_EMOJIS["Other"]

    def get_command_signature(self, command):
        """Returns the command signautre (prefix + command + usage)"""
        parent = command.full_parent_name
        alias = command.name if not parent else f"{parent} {command.name}"

        usage = self.get_decorator_var(command, "usage", default="<args>")

        return f"{self.prefix}{alias} {usage}"

    async def send_bot_help(self, mapping):
        """Sends help for all commands organized by category"""
        embed = Embed(
            title="KOᕼᗩKᑌ",
            description="Wasshoi~!\nType `-help <command>` to see more details about a particular command.",
            color=self.color,
        )
        embed.set_thumbnail(url=self.context.bot.user.display_avatar.url)
        embed.set_footer(text="v3.0a | <..> Mandatory | (...) optional")

        # Add commands based on category
        all_commands = []
        for _, cmds in mapping.items():
            filtered = await self.filter_commands(cmds, sort=True)
            all_commands.extend(filtered)

        categories = {}
        for cmd in all_commands:
            cat = self.get_command_category(cmd) if cmd.name != "help" else "Info"
            if cat not in categories:
                categories[cat] = []
            categories[cat].append(cmd)

        sorted_categories = sorted(categories.keys(), key=lambda x: (x == "Other", x))
        for cat in sorted_categories:
            cmds = categories[cat]
            emoji = self.get_category_emoji(cat)

            sorted_cmds = sorted(cmds, key=lambda c: (isinstance(c, commands.Group), c.name))

            cmd_list = []
            for cmd in sorted_cmds:
                cmd_listing = f"`{self.prefix}{cmd.name}`"
                cmd_list.append(cmd_listing)

            if cmd_list != []:
                embed.add_field(name=f"{emoji} __{cat}__", value="\n".join(cmd_list), inline=False)

        # Add source
        view = None
        if self.repo is not None:
            view = View()
            view.add_item(Button(label="Github", style=ButtonStyle.link, url=self.repo, emoji="🔗"))

        return await self.get_destination().send(embed=embed, view=view)

    async def send_command_help(self, command):
        """Sends help for one specfic command"""
        if command.name == "help":
            return await self.send_bot_help(self.get_bot_mapping())

        cat_name = self.get_command_category(command)
        emoji = self.get_category_emoji(cat_name)

        name = self.get_decorator_var(command, "title", default=command.name)
        desc = self.get_decorator_var(command, "description")
        if command.aliases:
            aliases = ", ".join([f"`{alias}`" for alias in command.aliases]).strip()
            desc += f"\n🔄 Aliases: {aliases}"

        usage = self.get_command_signature(command)

        embed = Embed(title=name, description=desc, color=self.color)

        embed.add_field(name="💡 Usage", value=f"`{usage}`", inline=False)

        footer = f"Category: {emoji} {cat_name}"
        if isinstance(command.parent, commands.Group):
            footer += f" | Group: {command.parent.name}"

        embed.set_footer(text=footer)

        return await self.get_destination().send(embed=embed)

    async def send_group_help(self, group):
        """Sends help for one group command"""
        cat_name = self.get_command_category(group)
        emoji = self.get_category_emoji(cat_name)

        name = self.get_decorator_var(group, "title", default=group.name)
        desc = self.get_decorator_var(group, "description")
        if group.aliases:
            aliases = ", ".join([f"`{alias}`" for alias in group.aliases]).strip()
            desc += f"\n🔄 Aliases: {aliases}"

        usage = self.get_command_signature(group)

        subcommands = []
        for cmd in group.commands:
            cmd_desc = self.get_decorator_var(cmd, "description")
            if cmd_desc is not None:
                if len(cmd_desc) > 60:
                    # Shorten description
                    i = cmd_desc.rfind(" ", start=50)
                    if i == -1:
                        # One long word
                        i = 55
                    cmd_desc = cmd_desc[:i] + " [...]"
                subcommands.append(f"`{cmd.name}` - {cmd_desc}")
            else:
                subcommands.append(f"`{cmd.name}`")
        subcommands = "\n".join(subcommands)

        embed = Embed(title=name, description=desc, color=self.color)

        embed.add_field(name="💡 Usage", value=f"`{usage}`", inline=False)

        embed.add_field(name="📖 Subcommands", value=subcommands, inline=False)

        footer = f"Category: {emoji} {cat_name}"
        if isinstance(group.parent, commands.Group):
            footer += f" | Group: {group.parent.name}"

        embed.set_footer(text=footer)

        return await self.get_destination().send(embed=embed)

    async def send_error_message(self, error):
        """Sends an error message"""
        embed = Embed(description=f"❌ {error}", color=self.color_error)
        return await self.get_destination().send(embed=embed)
