use ratatui::{prelude::*, symbols::border, widgets::*};

use crate::app::{App, AppState, Tab};

impl StatefulWidget for &App {
    type State = AppState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(1),
                Constraint::Length(10),
            ])
            .split(area);

        let tabs = Tabs::new(
            Tab::VALUES
                .iter()
                .enumerate()
                .map(|(i, tab)| format!("[{}] {}", i + 1, tab)),
        )
        .block(Block::bordered().border_type(BorderType::Rounded))
        .highlight_style(Style::default().bg(Color::Blue))
        .select(
            Tab::VALUES
                .iter()
                .position(|tab| tab == &self.current_screen)
                .unwrap_or(0),
        )
        .divider(symbols::line::VERTICAL);
        tabs.render(chunks[0], buf);

        let now_playing_block = Block::bordered()
            .title("Playing")
            .border_set(border::ROUNDED);

        let now_playing_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(20), Constraint::Min(1)])
            .split(now_playing_block.inner(chunks[2]));
        now_playing_block.render(chunks[2], buf);

        if let Some(image) = &mut state.now_playing.image {
            let stateful_image = ratatui_image::StatefulImage::default();
            stateful_image.render(now_playing_chunks[0], buf, image);
        }

        if let Some(entity_title) = &state.now_playing.entity_tittle {
            Text::raw(entity_title).render(now_playing_chunks[1], buf);
        }

        let block = Block::bordered()
            .title(self.current_screen.to_string())
            .border_set(border::ROUNDED);

        let tab_content = match self.current_screen {
            Tab::FavoriteAlbums => Text::from(vec![Line::from(vec!["albums".into()])]),
            Tab::FavoriteArtists => Text::from(vec![Line::from(vec!["artists".into()])]),
        };

        Paragraph::new(tab_content)
            .centered()
            .block(block)
            .render(chunks[1], buf);

        // match &self.current_screen {
        //     crate::app::Tab::NowPlaying => {
        //         let title = Line::from(" Now playing ".bold());
        //         let _block = Block::bordered()
        //             .title(title.centered())
        //             .border_set(border::ROUNDED);

        //         let image = self
        //             .now_playing
        //             .image
        //             .as_ref()
        //             .and_then(|image_url| fetch_image(image_url.clone()));

        //         if let Some(mut image) = image {
        //             let stateful_image = ratatui_image::StatefulImage::default();
        //             stateful_image.render(area, buf, &mut image);
        //         }

        //         // if let Some(track_title_text) = now_playing_state.entity_tittle.clone() {
        //         //     Paragraph::new(track_title_text)
        //         //         .centered()
        //         //         .block(block)
        //         //         .render(area, buf);
        //         // }
        //     }
        //     crate::app::Tab::FavoriteAlbums => {
        //         let title = Line::from(" Favorite albums ".bold());
        //         let instructions = Line::from(vec![
        //             " Decrement ".into(),
        //             "<Left>".blue().bold(),
        //             " Increment ".into(),
        //             "<Right>".blue().bold(),
        //             " Quit ".into(),
        //             "<Q> ".blue().bold(),
        //         ]);
        //         let block = Block::bordered()
        //             .title(title.centered())
        //             .title_bottom(instructions.centered())
        //             .border_set(border::THICK);

        //         let counter_text = Text::from(vec![Line::from(vec!["albums".into()])]);

        //         Paragraph::new(counter_text)
        //             .centered()
        //             .block(block)
        //             .render(area, buf);
        //     }
        //     crate::app::Tab::FavoriteArtists => {
        //         let title = Line::from(" Favorite artists ".bold());
        //         let instructions = Line::from(vec![
        //             " Decrement ".into(),
        //             "<Left>".blue().bold(),
        //             " Increment ".into(),
        //             "<Right>".blue().bold(),
        //             " Quit ".into(),
        //             "<Q> ".blue().bold(),
        //         ]);
        //         let block = Block::bordered()
        //             .title(title.centered())
        //             .title_bottom(instructions.centered())
        //             .border_set(border::THICK);

        //         let counter_text = Text::from(vec![Line::from(vec!["artists".into()])]);

        //         Paragraph::new(counter_text)
        //             .centered()
        //             .block(block)
        //             .render(area, buf);
        //     }
        // }
    }

    // fn render(self, area: Rect, buf: &mut Buffer)
    // where
    //     Self: Sized,
    // {
    //     let chunks = Layout::default()
    //         .direction(Direction::Vertical)
    //         .constraints([
    //             Constraint::Length(3),
    //             Constraint::Min(1),
    //             Constraint::Length(10),
    //         ])
    //         .split(area);

    //     let tabs = Tabs::new(
    //         Tab::VALUES
    //             .iter()
    //             .enumerate()
    //             .map(|(i, tab)| format!("[{}] {}", i + 1, tab)),
    //     )
    //     .block(Block::bordered().border_type(BorderType::Rounded))
    //     .highlight_style(Style::default().bg(Color::Blue))
    //     .select(
    //         Tab::VALUES
    //             .iter()
    //             .position(|tab| tab == &self.current_screen)
    //             .unwrap_or(0),
    //     )
    //     .divider(symbols::line::VERTICAL);
    //     tabs.render(chunks[0], buf);

    //     // TODO: Only if playing
    //     // let now_playing = Block::bordered()
    //     //     .title("Playing")
    //     //     .border_set(border::ROUNDED);

    //     if let Some(image) = &mut self.now_playing.image {
    //         image.render(chunks[2], buf);
    //     }

    //     // now_playing.render(chunks[2], buf);

    //     let block = Block::bordered()
    //         .title(self.current_screen.to_string())
    //         .border_set(border::ROUNDED);

    //     let tab_content = match self.current_screen {
    //         Tab::FavoriteAlbums => Text::from(vec![Line::from(vec!["albums".into()])]),
    //         Tab::FavoriteArtists => Text::from(vec![Line::from(vec!["artists".into()])]),
    //     };

    //     Paragraph::new(tab_content)
    //         .centered()
    //         .block(block)
    //         .render(chunks[1], buf);

    //     // match &self.current_screen {
    //     //     crate::app::Tab::NowPlaying => {
    //     //         let title = Line::from(" Now playing ".bold());
    //     //         let _block = Block::bordered()
    //     //             .title(title.centered())
    //     //             .border_set(border::ROUNDED);

    //     //         let image = self
    //     //             .now_playing
    //     //             .image
    //     //             .as_ref()
    //     //             .and_then(|image_url| fetch_image(image_url.clone()));

    //     //         if let Some(mut image) = image {
    //     //             let stateful_image = ratatui_image::StatefulImage::default();
    //     //             stateful_image.render(area, buf, &mut image);
    //     //         }

    //     //         // if let Some(track_title_text) = now_playing_state.entity_tittle.clone() {
    //     //         //     Paragraph::new(track_title_text)
    //     //         //         .centered()
    //     //         //         .block(block)
    //     //         //         .render(area, buf);
    //     //         // }
    //     //     }
    //     //     crate::app::Tab::FavoriteAlbums => {
    //     //         let title = Line::from(" Favorite albums ".bold());
    //     //         let instructions = Line::from(vec![
    //     //             " Decrement ".into(),
    //     //             "<Left>".blue().bold(),
    //     //             " Increment ".into(),
    //     //             "<Right>".blue().bold(),
    //     //             " Quit ".into(),
    //     //             "<Q> ".blue().bold(),
    //     //         ]);
    //     //         let block = Block::bordered()
    //     //             .title(title.centered())
    //     //             .title_bottom(instructions.centered())
    //     //             .border_set(border::THICK);

    //     //         let counter_text = Text::from(vec![Line::from(vec!["albums".into()])]);

    //     //         Paragraph::new(counter_text)
    //     //             .centered()
    //     //             .block(block)
    //     //             .render(area, buf);
    //     //     }
    //     //     crate::app::Tab::FavoriteArtists => {
    //     //         let title = Line::from(" Favorite artists ".bold());
    //     //         let instructions = Line::from(vec![
    //     //             " Decrement ".into(),
    //     //             "<Left>".blue().bold(),
    //     //             " Increment ".into(),
    //     //             "<Right>".blue().bold(),
    //     //             " Quit ".into(),
    //     //             "<Q> ".blue().bold(),
    //     //         ]);
    //     //         let block = Block::bordered()
    //     //             .title(title.centered())
    //     //             .title_bottom(instructions.centered())
    //     //             .border_set(border::THICK);

    //     //         let counter_text = Text::from(vec![Line::from(vec!["artists".into()])]);

    //     //         Paragraph::new(counter_text)
    //     //             .centered()
    //     //             .block(block)
    //     //             .render(area, buf);
    //     //     }
    //     // }
    // }
}
