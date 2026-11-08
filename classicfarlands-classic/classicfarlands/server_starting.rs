// ServerLifecycleEvents.SERVER_STARTING: nothing of the world is loaded yet, so the distance read
// here is the one the density registry and the noise use for the whole session.
fn server_starting(game: &mut crate::game::Game<'_>) {
    crate::game::port_classicfarlands_game::on_server_starting(game);
}
