use super::super::CanisterInputMessage;
use super::*;
use ic_test_utilities::types::{
    ids::{canister_test_id, message_test_id, user_test_id},
    messages::{IngressBuilder, RequestBuilder, ResponseBuilder},
};
use ic_types::time::current_time_and_expiry_time;
use std::convert::TryInto;

#[test]
/// Can push one request to the output queues.
fn can_push_output_request() {
    let this = canister_test_id(13);
    let mut queues = CanisterQueues::default();
    queues
        .push_output_request(RequestBuilder::default().sender(this).build())
        .unwrap();
}

#[test]
#[should_panic(expected = "pushing response into inexistent output queue")]
/// Cannot push response to output queues without pushing an input request
/// first.
fn cannot_push_output_response_without_input_request() {
    let this = canister_test_id(13);
    let mut queues = CanisterQueues::default();
    queues.push_output_response(ResponseBuilder::default().respondent(this).build());
}

#[test]
fn enqueuing_unexpected_response_does_not_panic() {
    let other = canister_test_id(14);
    let this = canister_test_id(13);
    let mut queues = CanisterQueues::default();
    // Enqueue a request to create a queue for `other`.
    queues
        .push_input(
            QueueIndex::from(0),
            RequestBuilder::default()
                .sender(other)
                .receiver(this)
                .build()
                .into(),
        )
        .unwrap();
    // Now `other` sends an unexpected `Response`.  We should return an error not
    // panic.
    queues
        .push_input(
            QUEUE_INDEX_NONE,
            ResponseBuilder::default()
                .respondent(other)
                .originator(this)
                .build()
                .into(),
        )
        .unwrap_err();
}

#[test]
/// Can push response to output queues after pushing input request.
fn can_push_output_response_after_input_request() {
    let this = canister_test_id(13);
    let other = canister_test_id(14);
    let mut queues = CanisterQueues::default();
    queues
        .push_input(
            QueueIndex::from(0),
            RequestBuilder::default()
                .sender(other)
                .receiver(this)
                .build()
                .into(),
        )
        .unwrap();
    queues.push_output_response(
        ResponseBuilder::default()
            .respondent(this)
            .originator(other)
            .build(),
    );
}

#[test]
/// Can push one request to the induction pool.
fn can_push_input_request() {
    let this = canister_test_id(13);
    let mut queues = CanisterQueues::default();
    queues
        .push_input(
            QueueIndex::from(0),
            RequestBuilder::default().receiver(this).build().into(),
        )
        .unwrap();
}

#[test]
/// Cannot push response to the induction pool without pushing output
/// request first.
fn cannot_push_input_response_without_output_request() {
    let this = canister_test_id(13);
    let mut queues = CanisterQueues::default();
    queues
        .push_input(
            QueueIndex::from(0),
            ResponseBuilder::default().originator(this).build().into(),
        )
        .unwrap_err();
}

#[test]
/// Can push response to input queues after pushing request to output
/// queues.
fn can_push_input_response_after_output_request() {
    let this = canister_test_id(13);
    let other = canister_test_id(14);
    let mut queues = CanisterQueues::default();
    queues
        .push_output_request(
            RequestBuilder::default()
                .sender(this)
                .receiver(other)
                .build(),
        )
        .unwrap();
    queues
        .push_input(
            QueueIndex::from(0),
            ResponseBuilder::default()
                .respondent(other)
                .originator(this)
                .build()
                .into(),
        )
        .unwrap();
}

#[test]
/// Enqueues 10 ingress messages and pops them.
fn test_message_picking_ingress_only() {
    let this = canister_test_id(13);

    let mut queues = CanisterQueues::default();
    assert!(queues.pop_input().is_none());

    for i in 0..10 {
        queues.push_ingress(Ingress {
            source: user_test_id(77),
            receiver: this,
            method_name: String::from("test"),
            method_payload: vec![i as u8],
            message_id: message_test_id(555),
            expiry_time: current_time_and_expiry_time().1,
        });
    }

    let mut expected_byte = 0;
    while queues.has_input() {
        match queues.pop_input().expect("could not pop a message") {
            CanisterInputMessage::Ingress(msg) => {
                assert_eq!(msg.method_payload, vec![expected_byte])
            }
            msg => panic!("unexpected message popped: {:?}", msg),
        }
        expected_byte += 1;
    }
    assert_eq!(10, expected_byte);

    assert!(queues.pop_input().is_none());
}

#[test]
/// Enqueues 3 requests for the same canister and consumes them.
fn test_message_picking_round_robin_on_one_queue() {
    let this = canister_test_id(13);
    let other = canister_test_id(14);

    let mut queues = CanisterQueues::default();
    assert!(queues.pop_input().is_none());

    let list = vec![(0, other), (1, other), (2, other)];
    for (ix, id) in list.iter() {
        queues
            .push_input(
                QueueIndex::from(*ix),
                RequestBuilder::default()
                    .sender(*id)
                    .receiver(this)
                    .build()
                    .into(),
            )
            .expect("could not push");
    }

    for _ in 0..list.len() {
        match queues.pop_input().expect("could not pop a message") {
            CanisterInputMessage::Request(msg) => assert_eq!(msg.sender, other),
            msg => panic!("unexpected message popped: {:?}", msg),
        }
    }

    assert!(!queues.has_input());
    assert!(queues.pop_input().is_none());
}

#[test]
/// Enqueues 3 requests and 1 response, then pops them and verifies the
/// expected order.
fn test_message_picking_round_robin() {
    let this = canister_test_id(13);
    let other_1 = canister_test_id(1);
    let other_2 = canister_test_id(2);
    let other_3 = canister_test_id(3);

    let mut queues = CanisterQueues::default();
    assert!(queues.pop_input().is_none());

    for (ix, id) in &[(0, other_1), (1, other_1), (0, other_3)] {
        queues
            .push_input(
                QueueIndex::from(*ix),
                RequestBuilder::default()
                    .sender(*id)
                    .receiver(this)
                    .build()
                    .into(),
            )
            .expect("could not push");
    }

    queues
        .push_output_request(
            RequestBuilder::default()
                .sender(this)
                .receiver(other_2)
                .build(),
        )
        .unwrap();
    // This succeeds because we pushed a request to other_2 to the output_queue
    // above which reserved a slot for the expected response here.
    queues
        .push_input(
            QueueIndex::from(0),
            ResponseBuilder::default()
                .respondent(other_2)
                .originator(this)
                .build()
                .into(),
        )
        .expect("could not push");

    queues.push_ingress(Ingress {
        source: user_test_id(77),
        receiver: this,
        method_name: String::from("test"),
        method_payload: Vec::new(),
        message_id: message_test_id(555),
        expiry_time: current_time_and_expiry_time().1,
    });

    /* POPPING */

    // Pop ingress first due to the round-robin across ingress and x-net messages
    match queues.pop_input().expect("could not pop a message") {
        CanisterInputMessage::Ingress(msg) => assert_eq!(msg.source, user_test_id(77)),
        msg => panic!("unexpected message popped: {:?}", msg),
    }

    // Pop request from other_1
    match queues.pop_input().expect("could not pop a message") {
        CanisterInputMessage::Request(msg) => assert_eq!(msg.sender, other_1),
        msg => panic!("unexpected message popped: {:?}", msg),
    }

    // Pop request from other_3
    match queues.pop_input().expect("could not pop a message") {
        CanisterInputMessage::Request(msg) => assert_eq!(msg.sender, other_3),
        msg => panic!("unexpected message popped: {:?}", msg),
    }

    // Pop response from other_2
    match queues.pop_input().expect("could not pop a message") {
        CanisterInputMessage::Response(msg) => assert_eq!(msg.respondent, other_2),
        msg => panic!("unexpected message popped: {:?}", msg),
    }

    // Pop request from other_1
    match queues.pop_input().expect("could not pop a message") {
        CanisterInputMessage::Request(msg) => assert_eq!(msg.sender, other_1),
        msg => panic!("unexpected message popped: {:?}", msg),
    }

    assert!(!queues.has_input());
    assert!(queues.pop_input().is_none());
}

#[test]
/// Enqueues 4 input requests across 3 canisters and consumes them, ensuring
/// correct round-robin scheduling.
fn test_input_scheduling() {
    let this = canister_test_id(13);
    let other_1 = canister_test_id(1);
    let other_2 = canister_test_id(2);
    let other_3 = canister_test_id(3);

    let mut queues = CanisterQueues::default();
    assert!(!queues.has_input());

    let push_input_from = |queues: &mut CanisterQueues, sender: &CanisterId, index: u64| {
        queues
            .push_input(
                QueueIndex::from(index),
                RequestBuilder::default()
                    .sender(*sender)
                    .receiver(this)
                    .build()
                    .into(),
            )
            .expect("could not push");
    };

    let assert_schedule = |queues: &CanisterQueues, expected_schedule: &[&CanisterId]| {
        let schedule: Vec<&CanisterId> = queues.input_schedule.iter().collect();
        assert_eq!(expected_schedule, schedule.as_slice());
    };

    let assert_sender = |sender: &CanisterId, message: CanisterInputMessage| match message {
        CanisterInputMessage::Request(req) => assert_eq!(*sender, req.sender),
        _ => unreachable!(),
    };

    push_input_from(&mut queues, &other_1, 0);
    assert_schedule(&queues, &[&other_1]);

    push_input_from(&mut queues, &other_2, 0);
    assert_schedule(&queues, &[&other_1, &other_2]);

    push_input_from(&mut queues, &other_1, 1);
    assert_schedule(&queues, &[&other_1, &other_2]);

    push_input_from(&mut queues, &other_3, 0);
    assert_schedule(&queues, &[&other_1, &other_2, &other_3]);

    assert_sender(&other_1, queues.pop_input().unwrap());
    assert_schedule(&queues, &[&other_2, &other_3, &other_1]);

    assert_sender(&other_2, queues.pop_input().unwrap());
    assert_schedule(&queues, &[&other_3, &other_1]);

    assert_sender(&other_3, queues.pop_input().unwrap());
    assert_schedule(&queues, &[&other_1]);

    assert_sender(&other_1, queues.pop_input().unwrap());
    assert_schedule(&queues, &[]);

    assert!(!queues.has_input());
}

#[test]
/// Enqueues 6 output requests across 3 canisters and consumes them.
fn test_output_into_iter() {
    let this = canister_test_id(13);
    let other_1 = canister_test_id(1);
    let other_2 = canister_test_id(2);
    let other_3 = canister_test_id(3);

    let canister_id = canister_test_id(1);
    let mut queues = CanisterQueues::default();
    assert_eq!(0, queues.output_into_iter(canister_id).count());

    let destinations = vec![other_1, other_2, other_1, other_3, other_2, other_1];
    for (i, id) in destinations.iter().enumerate() {
        queues
            .push_output_request(
                RequestBuilder::default()
                    .sender(this)
                    .receiver(*id)
                    .method_payload(vec![i as u8])
                    .build(),
            )
            .expect("could not push");
    }

    let expected = vec![
        (&other_1, 0, 0),
        (&other_1, 1, 2),
        (&other_1, 2, 5),
        (&other_2, 0, 1),
        (&other_2, 1, 4),
        (&other_3, 0, 3),
    ];
    assert_eq!(
        expected.len(),
        queues.clone().output_into_iter(this).count()
    );

    for (i, (qid, idx, msg)) in queues.output_into_iter(this).enumerate() {
        assert_eq!(this, qid.src_canister);
        assert_eq!(*expected[i].0, qid.dst_canister);
        assert_eq!(expected[i].1, idx.get());
        match msg {
            RequestOrResponse::Request(msg) => {
                assert_eq!(vec![expected[i].2], msg.method_payload)
            }
            msg => panic!("unexpected message popped: {:?}", msg),
        }
    }

    assert_eq!(0, queues.output_into_iter(canister_id).count());
}

#[test]
/// Tests that an encode-decode roundtrip yields a result equal to the
/// original (and the queue size metrics of an organically constructed
/// `CanisterQueues` match those of a deserialized one).
fn encode_roundtrip() {
    let mut queues = CanisterQueues::default();

    let this = canister_test_id(13);
    let other = canister_test_id(14);
    queues
        .push_input(
            QueueIndex::from(0),
            RequestBuilder::default().sender(this).build().into(),
        )
        .unwrap();
    queues
        .push_input(
            QueueIndex::from(0),
            RequestBuilder::default().sender(other).build().into(),
        )
        .unwrap();
    queues.pop_canister_input().unwrap();
    queues.push_ingress(IngressBuilder::default().receiver(this).build());

    let encoded: pb_queues::CanisterQueues = (&queues).into();
    let decoded = encoded.try_into().unwrap();

    assert_eq!(queues, decoded);
}

#[test]
/// Enqueues requests and responses into input and output queues, verifying that
/// input queue and memory usage stats are accurate along the way.
fn test_stats() {
    let this = canister_test_id(13);
    let other_1 = canister_test_id(1);
    let other_2 = canister_test_id(2);
    let other_3 = canister_test_id(3);
    const NAME: &str = "abcd";
    let iq_size: usize = InputQueue::new(DEFAULT_QUEUE_CAPACITY).calculate_size_bytes();
    let mut msg_size = [0; 6];

    let mut queues = CanisterQueues::default();
    let mut expected_iq_stats = InputQueuesStats::default();
    let mut expected_mu_stats = MemoryUsageStats::default();
    assert_eq!(expected_iq_stats, queues.input_queues_stats);
    assert_eq!(expected_mu_stats, queues.memory_usage_stats);

    // Push 3 requests into 3 input queues.
    for (i, sender) in [other_1, other_2, other_3].iter().enumerate() {
        let msg: RequestOrResponse = RequestBuilder::default()
            .sender(*sender)
            .receiver(this)
            .method_name(&NAME[0..i + 1]) // Vary request size.
            .build()
            .into();
        msg_size[i] = msg.count_bytes();
        queues
            .push_input(QUEUE_INDEX_NONE, msg)
            .expect("could not push");

        // Added a new input queue and `msg`.
        expected_iq_stats += InputQueuesStats {
            message_count: 1,
            size_bytes: iq_size + msg_size[i],
        };
        assert_eq!(expected_iq_stats, queues.input_queues_stats);
        // Pushed a request: one more reserved slot, no reserved response bytes.
        expected_mu_stats.reserved_slots += 1;
        assert_eq!(expected_mu_stats, queues.memory_usage_stats);
    }

    // Pop the first request we just pushed (as if it has started execution).
    match queues.pop_input().expect("could not pop a message") {
        CanisterInputMessage::Request(msg) => assert_eq!(msg.sender, other_1),
        msg => panic!("unexpected message popped: {:?}", msg),
    }
    // We've now removed all messages in the input queue from `other_1`, but the
    // queue is still there.
    expected_iq_stats -= InputQueuesStats {
        message_count: 1,
        size_bytes: msg_size[0],
    };
    assert_eq!(expected_iq_stats, queues.input_queues_stats);
    // Memory usage stats are unchanged, as the reservation is still there.
    assert_eq!(expected_mu_stats, queues.memory_usage_stats);

    // And push a matching output response.
    let msg = ResponseBuilder::default()
        .respondent(this)
        .originator(other_1)
        .build();
    msg_size[3] = msg.count_bytes();
    queues.push_output_response(msg);
    // Input queue stats are unchanged.
    assert_eq!(expected_iq_stats, queues.input_queues_stats);
    // Consumed a reservation and added a response.
    expected_mu_stats += MemoryUsageStats {
        reserved_slots: -1,
        responses_size_bytes: msg_size[3],
        oversized_requests_extra_bytes: 0,
        transient_stream_responses_size_bytes: 0,
    };
    assert_eq!(expected_mu_stats, queues.memory_usage_stats);

    // Push an oversized request into the same output queue (to `other_1`).
    let msg = RequestBuilder::default()
        .sender(this)
        .receiver(other_1)
        .method_name(NAME)
        .method_payload(vec![13; MAX_RESPONSE_COUNT_BYTES])
        .build();
    msg_size[4] = msg.count_bytes();
    queues.push_output_request(msg).unwrap();
    // One more reserved slot, no reserved response bytes, oversized request.
    expected_mu_stats.reserved_slots += 1;
    expected_mu_stats.oversized_requests_extra_bytes += msg_size[4] - MAX_RESPONSE_COUNT_BYTES;
    assert_eq!(expected_iq_stats, queues.input_queues_stats);
    assert_eq!(expected_mu_stats, queues.memory_usage_stats);

    // Call `output_into_iter()` but don't consume any messages.
    #[allow(unused_must_use)]
    {
        queues.output_into_iter(this);
    }
    // Stats should stay unchanged.
    assert_eq!(expected_iq_stats, queues.input_queues_stats);
    assert_eq!(expected_mu_stats, queues.memory_usage_stats);

    // Call `output_into_iter()` and consume a single message.
    match queues
        .output_into_iter(this)
        .next()
        .expect("could not pop a message")
    {
        (_, _, RequestOrResponse::Response(msg)) => assert_eq!(msg.originator, other_1),
        msg => panic!("unexpected message popped: {:?}", msg),
    }
    // No input queue changes.
    assert_eq!(expected_iq_stats, queues.input_queues_stats);
    // But we've consumed the response.
    expected_mu_stats.responses_size_bytes -= msg_size[3];
    assert_eq!(expected_mu_stats, queues.memory_usage_stats);

    // Consume the outgoing request.
    match queues
        .output_into_iter(this)
        .next()
        .expect("could not pop a message")
    {
        (_, _, RequestOrResponse::Request(msg)) => assert_eq!(msg.receiver, other_1),
        msg => panic!("unexpected message popped: {:?}", msg),
    }
    // No input queue changes.
    assert_eq!(expected_iq_stats, queues.input_queues_stats);
    // Oversized request was popped.
    expected_mu_stats.oversized_requests_extra_bytes -= msg_size[4] - MAX_RESPONSE_COUNT_BYTES;
    assert_eq!(expected_mu_stats, queues.memory_usage_stats);

    // Ensure no more outgoing messages.
    assert!(queues.output_into_iter(this).next().is_none());

    // And enqueue a matching incoming response.
    let msg: RequestOrResponse = ResponseBuilder::default()
        .respondent(other_1)
        .originator(this)
        .build()
        .into();
    msg_size[5] = msg.count_bytes();
    queues
        .push_input(QUEUE_INDEX_NONE, msg)
        .expect("could not push");
    // Added a new input message.
    expected_iq_stats += InputQueuesStats {
        message_count: 1,
        size_bytes: msg_size[5],
    };
    assert_eq!(expected_iq_stats, queues.input_queues_stats);
    // Consumed one reservation, added some response bytes.
    expected_mu_stats += MemoryUsageStats {
        reserved_slots: -1,
        responses_size_bytes: msg_size[5],
        oversized_requests_extra_bytes: 0,
        transient_stream_responses_size_bytes: 0,
    };
    assert_eq!(expected_mu_stats, queues.memory_usage_stats);

    // Pop everything.

    // Pop request from other_2
    match queues.pop_input().expect("could not pop a message") {
        CanisterInputMessage::Request(msg) => assert_eq!(msg.sender, other_2),
        msg => panic!("unexpected message popped: {:?}", msg),
    }
    // Removed message.
    expected_iq_stats -= InputQueuesStats {
        message_count: 1,
        size_bytes: msg_size[1],
    };
    assert_eq!(expected_iq_stats, queues.input_queues_stats);
    // Memory usage stats unchanged, as the reservation is still there.
    assert_eq!(expected_mu_stats, queues.memory_usage_stats);

    // Pop request from other_3
    match queues.pop_input().expect("could not pop a message") {
        CanisterInputMessage::Request(msg) => assert_eq!(msg.sender, other_3),
        msg => panic!("unexpected message popped: {:?}", msg),
    }
    // Removed message.
    expected_iq_stats -= InputQueuesStats {
        message_count: 1,
        size_bytes: msg_size[2],
    };
    assert_eq!(expected_iq_stats, queues.input_queues_stats);
    // Memory usage stats unchanged, as the reservation is still there.
    assert_eq!(expected_mu_stats, queues.memory_usage_stats);

    // Pop response from other_1
    match queues.pop_input().expect("could not pop a message") {
        CanisterInputMessage::Response(msg) => assert_eq!(msg.respondent, other_1),
        msg => panic!("unexpected message popped: {:?}", msg),
    }
    // Removed message.
    expected_iq_stats -= InputQueuesStats {
        message_count: 1,
        size_bytes: msg_size[5],
    };
    assert_eq!(expected_iq_stats, queues.input_queues_stats);
    // We have consumed the response.
    expected_mu_stats.responses_size_bytes -= msg_size[5];
    assert_eq!(expected_mu_stats, queues.memory_usage_stats);
}

#[test]
/// Enqueues requests and responses into input and output queues, verifying that
/// input queue and memory usage stats are accurate along the way.
fn test_stats_induct_message_to_self() {
    let this = canister_test_id(13);
    let iq_size: usize = InputQueue::new(DEFAULT_QUEUE_CAPACITY).calculate_size_bytes();

    let mut queues = CanisterQueues::default();
    let mut expected_iq_stats = InputQueuesStats::default();
    let mut expected_mu_stats = MemoryUsageStats::default();
    assert_eq!(expected_iq_stats, queues.input_queues_stats);
    assert_eq!(expected_mu_stats, queues.memory_usage_stats);

    // No messages to induct.
    assert!(queues.induct_message_to_self(this).is_none());

    // Push a request to self.
    let request = RequestBuilder::default()
        .sender(this)
        .receiver(this)
        .method_name("self")
        .build();
    let request_size = request.count_bytes();
    queues.push_output_request(request).expect("could not push");

    // New input queue was created.
    expected_iq_stats.size_bytes += iq_size;
    assert_eq!(expected_iq_stats, queues.input_queues_stats);
    // Pushed a request: one more reserved slot, no reserved response bytes.
    expected_mu_stats.reserved_slots += 1;
    assert_eq!(expected_mu_stats, queues.memory_usage_stats);

    // Induct request.
    assert!(queues.induct_message_to_self(this).is_some());

    // Request is now in the input queue.
    expected_iq_stats += InputQueuesStats {
        message_count: 1,
        size_bytes: request_size,
    };
    assert_eq!(expected_iq_stats, queues.input_queues_stats);
    // We now have reservations (for the same request) in both the input and the
    // output queue.
    expected_mu_stats.reserved_slots += 1;
    assert_eq!(expected_mu_stats, queues.memory_usage_stats);

    // Pop the request (as if we were executing it).
    queues.pop_input().expect("could not pop request");
    // Request consumed.
    expected_iq_stats -= InputQueuesStats {
        message_count: 1,
        size_bytes: request_size,
    };
    assert_eq!(expected_iq_stats, queues.input_queues_stats);
    // Memory usage stats unchanged, as the reservations are still there.
    assert_eq!(expected_mu_stats, queues.memory_usage_stats);

    // And push a matching output response.
    let response = ResponseBuilder::default()
        .respondent(this)
        .originator(this)
        .build();
    let response_size = response.count_bytes();
    queues.push_output_response(response);
    // Input queue stats are unchanged.
    assert_eq!(expected_iq_stats, queues.input_queues_stats);
    // Consumed output queue reservation and added a response.
    expected_mu_stats += MemoryUsageStats {
        reserved_slots: -1,
        responses_size_bytes: response_size,
        oversized_requests_extra_bytes: 0,
        transient_stream_responses_size_bytes: 0,
    };
    assert_eq!(expected_mu_stats, queues.memory_usage_stats);

    // Induct the response.
    assert!(queues.induct_message_to_self(this).is_some());

    // Response is now in the input queue.
    expected_iq_stats += InputQueuesStats {
        message_count: 1,
        size_bytes: response_size,
    };
    assert_eq!(expected_iq_stats, queues.input_queues_stats);
    // Consumed input queue reservation but response is still there (in input queue
    // now).
    expected_mu_stats.reserved_slots -= 1;
    assert_eq!(expected_mu_stats, queues.memory_usage_stats);

    // Pop the response.
    queues.pop_input().expect("could not pop response");
    // Response consumed.
    expected_iq_stats -= InputQueuesStats {
        message_count: 1,
        size_bytes: response_size,
    };
    assert_eq!(expected_iq_stats, queues.input_queues_stats);
    // Zero response bytes, zero reservations.
    expected_mu_stats.responses_size_bytes -= response_size;
    assert_eq!(expected_mu_stats, queues.memory_usage_stats);
}
