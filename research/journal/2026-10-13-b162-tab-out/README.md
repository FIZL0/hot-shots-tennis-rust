# B162 tab-out disconnect
ggrs' 2 s disconnect timeout dropped a player whose frame loop stopped (hidden tab, minimised window); now 5 min, with
the others waiting on "Waiting for …" and a countdown. → 1-KEEPALIVE-FINAL.md

B196's silent-peer recovery then took a tabbed-out player for gone after 3 s (20 s wait, bot/lobby). A keeper off the
frame loop (`net/keeper.rs`) now says `Msg::Hidden` while the player has switched away; everyone holds the match with
"… switched away" for up to 5 min, and it plays on when they're back. → 2-HIDDEN-FINAL.md
