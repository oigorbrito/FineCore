/// A generated graph, with everything the checks need to know about it.
struct Shape {
    stages: Vec<Value>,
    /// Dependencies per stage id, after map expansion is accounted for.
    needs: BTreeMap<String, Vec<String>>,
    /// Stage ids that must each execute exactly once.
    expected: Vec<String>,
    /// Stages on the longest dependency chain. With a fixed model delay this
    /// gives the run's nominal cost, which measured service time is compared
    /// against to tell scheduling saturation from engine overhead.
    depth: usize,
    /// Stage ids whose answers the run must return as terminal outputs.
    terminal: Vec<String>,
}

fn stage(id: &str, prompt: String, needs: &[String]) -> Value {
    let mut value = json!({"id": id, "prompt": prompt});
    if !needs.is_empty() {
        value["needs"] = json!(needs);
    }
    value
}

/// `STAGE=id|NEEDS=a,b|a={{ stages.a.output }}|b={{ stages.b.output }}`
fn prompt_for(id: &str, needs: &[String]) -> String {
    let mut prompt = format!("STAGE={id}|NEEDS={}", needs.join(","));
    for need in needs {
        prompt.push_str(&format!("|{need}={{{{ stages.{need}.output }}}}"));
    }
    prompt
}

fn chain(n: usize) -> Shape {
    let ids: Vec<String> = (0..n).map(|index| format!("s{index}")).collect();
    let mut stages = Vec::with_capacity(n);
    let mut needs = BTreeMap::new();
    for (index, id) in ids.iter().enumerate() {
        let deps: Vec<String> = ids
            .get(index.wrapping_sub(1))
            .cloned()
            .into_iter()
            .collect();
        let deps = if index == 0 { Vec::new() } else { deps };
        stages.push(stage(id, prompt_for(id, &deps), &deps));
        needs.insert(id.clone(), deps);
    }
    Shape {
        stages,
        needs,
        expected: ids.clone(),
        depth: n,
        terminal: vec![ids[n - 1].clone()],
    }
}

/// One root, `n - 2` independent middles, one join: the shape a workflow is
/// usually drawn as, and the one the demo graph used.
fn diamond(n: usize) -> Shape {
    let width = n.saturating_sub(2).max(1);
    let root = "root".to_string();
    let mut stages = vec![stage(&root, prompt_for(&root, &[]), &[])];
    let mut needs = BTreeMap::from([(root.clone(), Vec::new())]);
    let mut expected = vec![root.clone()];
    let mut middles = Vec::with_capacity(width);
    for index in 0..width {
        let id = format!("m{index}");
        let deps = vec![root.clone()];
        stages.push(stage(&id, prompt_for(&id, &deps), &deps));
        needs.insert(id.clone(), deps);
        expected.push(id.clone());
        middles.push(id);
    }
    // The join names only two dependencies in its template: a prompt carrying
    // every upstream answer would measure string building, not scheduling.
    let join = "join".to_string();
    let templated: Vec<String> = middles.iter().take(2).cloned().collect();
    let mut join_stage = stage(&join, prompt_for(&join, &templated), &middles);
    join_stage["needs"] = json!(middles);
    stages.push(join_stage);
    needs.insert(join.clone(), middles);
    expected.push(join.clone());
    Shape {
        stages,
        needs,
        expected,
        depth: 3,
        terminal: vec![join],
    }
}

/// `n - 1` independent stages feeding one join: admission cost separated from
/// the settle that runs once with every dependency already satisfied.
fn wide_join(n: usize) -> Shape {
    let width = n.saturating_sub(1).max(1);
    let mut stages = Vec::with_capacity(n);
    let mut needs = BTreeMap::new();
    let mut expected = Vec::with_capacity(n);
    let mut roots = Vec::with_capacity(width);
    for index in 0..width {
        let id = format!("w{index}");
        stages.push(stage(&id, prompt_for(&id, &[]), &[]));
        needs.insert(id.clone(), Vec::new());
        expected.push(id.clone());
        roots.push(id);
    }
    let join = "join".to_string();
    let templated: Vec<String> = roots.iter().take(2).cloned().collect();
    let mut join_stage = stage(&join, prompt_for(&join, &templated), &roots);
    join_stage["needs"] = json!(roots);
    stages.push(join_stage);
    needs.insert(join.clone(), roots);
    expected.push(join.clone());
    Shape {
        stages,
        needs,
        expected,
        depth: 2,
        terminal: vec![join],
    }
}

/// A source array, a map over it, and a join that consumes the barrier: the
/// only shape whose stage count is decided by the engine rather than the graph.
fn fanout(n: usize) -> Shape {
    let items = n.saturating_sub(3).max(1);
    let source = "src".to_string();
    let map = "fan".to_string();
    let join = "join".to_string();
    let stages = vec![
        json!({
            "id": source,
            "prompt": format!("STAGE={source}|NEEDS=|EMIT_ITEMS={items}"),
            "schema": "string[]",
        }),
        json!({
            "id": map,
            "kind": "map",
            "over": source,
            "prompt": format!("STAGE={map}|NEEDS=|ITEM={{{{ item }}}}"),
        }),
        json!({
            "id": join,
            "needs": [map],
            "prompt": format!(
                "STAGE={join}|NEEDS=|ITEMS={items}|JOINED={{{{ stages.{map}.output }}}}"
            ),
        }),
    ];
    let mut needs = BTreeMap::from([(source.clone(), Vec::new())]);
    let mut expected = vec![source.clone()];
    for index in 0..items {
        let child = format!("{map}[{index}]");
        needs.insert(child.clone(), vec![source.clone()]);
        expected.push(child);
    }
    // The join waits on the barrier, which waits on every child.
    let children: Vec<String> = (0..items).map(|index| format!("{map}[{index}]")).collect();
    needs.insert(join.clone(), children);
    expected.push(join.clone());
    Shape {
        stages,
        needs,
        expected,
        depth: 3,
        terminal: vec![join],
    }
}

/// `layers` waves of `width`, each stage depending on two of the wave before:
/// dependencies that actually constrain the schedule without quadratic edges.
fn mesh(n: usize) -> Shape {
    let width = (n as f64).sqrt().round().max(2.0) as usize;
    let layers = n.div_ceil(width).max(2);
    let mut stages = Vec::new();
    let mut needs = BTreeMap::new();
    let mut expected = Vec::new();
    let mut previous: Vec<String> = Vec::new();
    for layer in 0..layers {
        let mut current = Vec::with_capacity(width);
        for column in 0..width {
            let id = format!("l{layer}c{column}");
            let deps: Vec<String> = if previous.is_empty() {
                Vec::new()
            } else {
                let mut deps = vec![previous[column % previous.len()].clone()];
                let second = previous[(column + 1) % previous.len()].clone();
                if second != deps[0] {
                    deps.push(second);
                }
                deps
            };
            stages.push(stage(&id, prompt_for(&id, &deps), &deps));
            needs.insert(id.clone(), deps);
            expected.push(id.clone());
            current.push(id);
        }
        previous = current;
    }
    Shape {
        stages,
        needs,
        expected,
        depth: layers,
        terminal: previous,
    }
}

fn build(shape: &str, n: usize) -> Shape {
    match shape {
        "chain" => chain(n.max(1)),
        "diamond" => diamond(n.max(3)),
        "wide-join" => wide_join(n.max(2)),
        "fanout" => fanout(n.max(4)),
        "mesh" => mesh(n.max(4)),
        other => panic!("unknown shape `{other}`"),
    }
}
