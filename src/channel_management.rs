use chrono::offset::Local;
use chrono::{Datelike, Duration, Weekday};
use discord::model::{ChannelId, ChannelType, Message, PublicChannel, ServerInfo, UserId};
use discord::{Discord, GetMessages};
use rand::{thread_rng, Rng};

const ME: UserId = UserId(include!("bot_id.txt"));

pub fn it_is_wednesday_my_dudes(discord: &Discord, server: &ServerInfo) {
    println!("Is it wednesday my dudes? : {}", server.name);
    let now = Local::now();
    if now.weekday() == Weekday::Wed {
        match discord.get_server_channels(server.id) {
            Err(err) => {
                println!("Error when retrieving channels: {:?}", err);
            }
            Ok(channels_query) => {
                let channel = channels_query
                    .iter()
                    .filter(|c| c.name == "announcements")
                    .nth(0);
                if let Some(channel) = channel {
                    println!("It's wednesday my dudes!");
                    let rand_num = thread_rng().gen_range(0, 5);
                    let message = match rand_num {
                        0 => "https://tinyurl.com/ybvjxvad",
                        1 => "https://imgflip.com/i/2isgmb",
                        2 => "https://i.imgur.com/KS8LM6i_d.jpg",
                        3 => "https://youtu.be/du-TY1GUFGk",
                        4 => "https://tinyurl.com/y93yg2ry",
                        _ => unreachable!(),
                    };
                    let result = discord.send_message(channel.id, message, "", false);
                    if result.is_err() {
                        println!(
                            "Failed to send wednesday message to channel: {}",
                            &channel.id
                        );
                    }
                } else {
                    println!("announcements- not found.");
                }
            }
        }
    }
}

pub fn clear_old_channels(discord: &Discord, server: &ServerInfo) {
    println!("Clearing old channels on server: {}", server.name);
    let channels_query = discord.get_server_channels(server.id);
    match channels_query {
        Err(err) => {
            println!("Error when retrieving channels: {:?}", err);
        }
        Ok(channels) => {
            let permanent_category = channels
                .iter()
                .filter(|c| {
                    c.kind == ChannelType::Category && c.name.to_lowercase().contains("permanent")
                }).next()
                .expect("No permanent category found, crashing.")
                .id;
            for channel in &channels {
                if channel.parent_id != Some(permanent_category) {
                    println!("Found temporary channel: {}", channel.name);
                    process_temp_channel(discord, channel);
                } else {
                    println!("Found permanent channel: {}", channel.name);
                    println!("\tSkipping.");
                }
            }
        }
    }
}

fn process_temp_channel(discord: &Discord, channel: &PublicChannel) {
    println!("{} is channel type {:?}.", channel.name, channel.kind);
    if channel.kind == ChannelType::Text {
        process_temp_text_channel(discord, channel);
    }
}

fn process_temp_text_channel(discord: &Discord, channel: &PublicChannel) {
    let days_old = get_channel_inactive_duration(discord, channel).num_days();
    if days_old == 6 {
        send_delete_warning(discord, channel.id);
    } else if days_old >= 7 {
        // Never delete a channel on which a warning hasn't been sent.
        match get_warning(discord, channel) {
            Some(warning) => {
                // 22 is intentional here as exactly 24 hours almost never happens.
                if (Local::now().signed_duration_since(warning.timestamp)).num_hours() >= 22 {
                    println!("Warning found and it is at least 22 hours old.  Deleting channel.");
                    let result = discord.delete_channel(channel.id);
                    if result.is_err() {
                        println!("Failed to delete channel: {}", channel.name);
                    }
                }
                // else warning is not old enough yet, don't delete.
            }
            None => {
                println!("Would normally delete this now but no warning has been sent.");
                send_delete_warning(discord, channel.id);
            }
        }
    }
}

fn get_warning(discord: &Discord, channel: &PublicChannel) -> Option<Message> {
    let last_msg_query = discord.get_messages(channel.id, GetMessages::MostRecent, Some(1));
    if let Err(err) = last_msg_query {
        println!("Error retrieving most recent message: {:?}", err);
        send_filler_message(discord, channel.id);
        return None;
    } else {
        let last_msg_vec = last_msg_query.unwrap();
        if last_msg_vec.len() == 0 {
            println!("No messages found in channel.  Posting one.");
            send_filler_message(discord, channel.id);
            return None;
        } else {
            println!("Got most recent message.  Checking if warning.");
            let last_msg = last_msg_vec[0].clone();
            if message_is_warning(&last_msg) {
                return Some(last_msg);
            } else {
                return None;
            }
        }
    }
}

fn message_is_warning(message: &Message) -> bool {
    message.author.id == ME && message
        .content
        .starts_with("WARNING CHANNEL DELETION IMMINENT!")
}

fn get_channel_inactive_duration(discord: &Discord, channel: &PublicChannel) -> Duration {
    // Get the most recent message from someone other than gsquire.
    // If no such message exists then use one from gsquire.
    // If there are no messages on this channel at all, post one.
    let last_msg_query = discord.get_messages(channel.id, GetMessages::MostRecent, Some(1));
    if let Err(err) = last_msg_query {
        println!(
            "Error retrieving most recent message posting one.: {:?}",
            err
        );
        send_filler_message(discord, channel.id);
        return Duration::days(0);
    } else {
        let last_msg_vec = last_msg_query.unwrap();
        if last_msg_vec.len() == 0 {
            println!("No messages found in channel.  Posting one.");
            send_filler_message(discord, channel.id);
            return Duration::days(0);
        } else {
            println!("Got most recent message..");
            let mut best_msg = last_msg_vec[0].clone();
            let mut last_is_warning = message_is_warning(&best_msg);
            let mut last_msg = best_msg.clone();
            // If this was sent by gsquire try and find one that isn't.
            'search: while best_msg.author.id == ME {
                println!(
                    "Message id {} is from me, getting the one before it.",
                    best_msg.id
                );
                let msg_query =
                    discord.get_messages(channel.id, GetMessages::Before(best_msg.id), Some(1));
                if let Err(err) = msg_query {
                    println!("Error on getting message before current message.");
                    println!("Error text: {:?}", err);
                    break 'search;
                } else {
                    let msg_query_vec = msg_query.unwrap();
                    if msg_query_vec.len() == 0 {
                        println!("No message was sent by anyone other than me.");
                        break 'search;
                    } else {
                        best_msg = msg_query_vec[0].clone();
                        // In the event that gsquire is the only sender on this channel gsquire
                        // should not use its own warning message to determine the age of the channel.
                        // Otherwise gsquire will continue warning indefinitely but never actually
                        // delete the channel.  This channel will likely contain a filler message
                        // posted by gsquire for the purpose of determining channel age.
                        // This code will likely grab that filler message.
                        if last_is_warning && !message_is_warning(&best_msg) {
                            last_msg = best_msg.clone();
                            last_is_warning = false;
                        }
                    }
                }
            }

            // If all messages in channel were sent by gsquire, use the most recent one to
            // determine length of inactivity.
            if best_msg.author.id == ME {
                best_msg = last_msg;
            }
            println!("Found good message, proceeding.");
            println!(
                "Timestamp of message being evaluated is: {}",
                best_msg.timestamp
            );
            return Local::now().signed_duration_since(best_msg.timestamp);
        }
    }
}

fn send_filler_message(discord: &Discord, channel_id: ChannelId) {
    println!("Sending filler message.");
    let result = discord.send_message(channel_id, include_str!("filler_message.txt"), "", false);
    if result.is_err() {
        println!("Failed to send filler message to channel: {}", channel_id);
    }
}

fn send_delete_warning(discord: &Discord, channel_id: ChannelId) {
    println!("Sending warning message.");
    let result = discord.send_message(channel_id, include_str!("delete_warning.txt"), "", false);
    if result.is_err() {
        println!("Failed to send warning message to channel: {}", channel_id);
    }
}
