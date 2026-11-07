"""Finder layout stored in .DS_Store, without AppleScript or a Finder session."""
import os

app = os.path.abspath(defines['app'])
background = os.path.abspath(defines['background'])
icon = os.path.abspath(defines['icon'])
files = [app]
symlinks = {'Aplicaciones': '/Applications'}
format = 'UDZO'
filesystem = 'HFS+'
volume_name = 'XD2Figma'
window_rect = ((180, 140), (760, 500))
icon_locations = {'XD2Figma.app': (212, 226), 'Aplicaciones': (548, 226)}
icon_size = 108
text_size = 13
hide_extensions = ['XD2Figma.app']
show_status_bar = False
show_toolbar = False
show_sidebar = False
show_pathbar = False
show_tab_view = False
show_icon_preview = False
show_item_info = False
include_icon_view_settings = True
include_list_view_settings = False
default_view = 'icon-view'
arrange_by = None
