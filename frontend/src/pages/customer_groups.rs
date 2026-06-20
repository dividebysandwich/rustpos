use leptos::prelude::*;
use uuid::Uuid;

use crate::i18n::I18n;
use crate::models::*;
use crate::pages::keyboard::{scroll_page_to_top, OnScreenKeyboard};
use crate::server_fns::*;

#[component]
pub fn CustomerGroupsPage() -> impl IntoView {
    let i18n = expect_context::<RwSignal<I18n>>();
    let (authorized, set_authorized) = signal(false);
    Effect::new(move || {
        leptos::task::spawn_local(async move {
            match get_current_user().await {
                Ok(Some(u)) if u.role == "admin" => set_authorized.set(true),
                _ => {
                    #[cfg(target_arch = "wasm32")]
                    { let _ = web_sys::window().unwrap().location().set_href("/login"); }
                }
            }
        });
    });

    let (groups, set_groups) = signal(Vec::<CustomerGroup>::new());
    let (editing_group, set_editing_group) = signal(Option::<CustomerGroup>::None);
    let (creating_group, set_creating_group) = signal(false);
    let (deleting_group, set_deleting_group) = signal(Option::<(Uuid, String)>::None);

    let (name, set_name) = signal(String::new());

    // On-screen keyboard (hidden on mobile via CSS)
    let (kb_open, set_kb_open) = signal(false);
    let (kb_shift, set_kb_shift) = signal(false);

    let on_kb_key = move |key: String| {
        match key.as_str() {
            "Backspace" => { set_name.update(|s| { s.pop(); }); }
            "Enter" => { set_kb_open.set(false); }
            "Shift" => { set_kb_shift.update(|s| *s = !*s); }
            "Space" => { set_name.update(|s| s.push(' ')); }
            ch => {
                let ch = if kb_shift.get() { ch.to_uppercase() } else { ch.to_lowercase() };
                set_name.update(|s| s.push_str(&ch));
            }
        }
    };

    let (reload, set_reload) = signal(0u32);
    Effect::new(move || {
        reload.get();
        leptos::task::spawn_local(async move {
            if let Ok(g) = fetch_customer_groups().await { set_groups.set(g); }
        });
    });

    let start_edit = move |group: CustomerGroup| {
        set_kb_open.set(false);
        scroll_page_to_top();
        set_name.set(group.name.clone());
        set_editing_group.set(Some(group));
        set_creating_group.set(false);
    };

    let start_create = move |_| {
        set_kb_open.set(false);
        set_name.set(String::new());
        set_creating_group.set(true);
        set_editing_group.set(None);
    };

    let save_group = move |_| {
        let editing = editing_group.get();
        let creating = creating_group.get();
        let n = name.get();
        if n.trim().is_empty() { return; }
        if creating {
            leptos::task::spawn_local(async move {
                if create_customer_group(n).await.is_ok() {
                    set_creating_group.set(false);
                    set_reload.update(|v| *v += 1);
                }
            });
        } else if let Some(group) = editing {
            let gid = group.id;
            leptos::task::spawn_local(async move {
                if update_customer_group(gid, n).await.is_ok() {
                    set_editing_group.set(None);
                    set_reload.update(|v| *v += 1);
                }
            });
        }
    };

    let cancel_edit = move |_| {
        set_kb_open.set(false);
        set_editing_group.set(None);
        set_creating_group.set(false);
        set_name.set(String::new());
    };

    let confirm_delete = move |id: Uuid, name: String| { set_deleting_group.set(Some((id, name))); };
    let delete_group_handler = move |_| {
        if let Some((id, _)) = deleting_group.get() {
            leptos::task::spawn_local(async move {
                if delete_customer_group(id).await.is_ok() {
                    set_deleting_group.set(None);
                    set_reload.update(|v| *v += 1);
                }
            });
        }
    };
    let cancel_delete = move |_| { set_deleting_group.set(None); };

    view! {
        <Show when=move || authorized.get() fallback=move || view! { <div class="loading">{move || i18n.get().t("general.loading")}</div> }>
        <div>
            <div class="page-header">
                <h2>{move || i18n.get().t("groups.title")}</h2>
                <div class="page-header-actions">
                    <button class="btn-primary" on:click=start_create
                        disabled=move || editing_group.get().is_some() || creating_group.get()
                    >{move || i18n.get().t("groups.add")}</button>
                </div>
            </div>

            <Show when=move || deleting_group.get().is_some() fallback=|| ()>
                {move || {
                    deleting_group.get().map(|(_, group_name)| {
                        let i = i18n.get();
                        let confirm_msg = i.t("groups.confirm_delete").replace("{name}", &group_name);
                        view! {
                            <div class="modal-overlay">
                                <div class="confirmation-modal">
                                    <h3>{i.t("general.confirm_delete")}</h3>
                                    <p>{confirm_msg}</p>
                                    <p class="warning-text">{i.t("groups.delete_warning")}</p>
                                    <div class="modal-actions">
                                        <button class="btn-danger" on:click=delete_group_handler>{i.t("general.delete")}</button>
                                        <button class="btn-secondary" on:click=cancel_delete>{i.t("general.cancel")}</button>
                                    </div>
                                </div>
                            </div>
                        }
                    })
                }}
            </Show>

            <Show when=move || editing_group.get().is_some() || creating_group.get() fallback=|| ()>
                <div class="edit-form">
                    <h3>{move || if creating_group.get() { i18n.get().t("groups.create") } else { i18n.get().t("groups.edit") }}</h3>
                    <div class="form-grid">
                        <div class="form-group">
                            <label>{move || i18n.get().t("groups.name")}</label>
                            <div class="admin-input-row">
                                <input type="text" value=move || name.get()
                                    on:focus=move |_| { set_kb_open.set(true); set_kb_shift.set(false); }
                                    on:input=move |ev| set_name.set(event_target_value(&ev)) />
                            </div>
                        </div>
                    </div>
                    <Show when=move || kb_open.get() fallback=|| ()>
                        <OnScreenKeyboard on_key=on_kb_key shift=kb_shift i18n=i18n />
                    </Show>
                    <div class="form-actions">
                        <button class="btn-success" on:click=save_group>{move || i18n.get().t("general.save")}</button>
                        <button class="btn-secondary" on:click=cancel_edit>{move || i18n.get().t("general.cancel")}</button>
                    </div>
                </div>
            </Show>

            <Show when=move || groups.get().is_empty() fallback=|| ()>
                <p class="text-muted">{move || i18n.get().t("groups.none")}</p>
            </Show>

            <table class="data-table">
                <thead><tr><th>{move || i18n.get().t("groups.name")}</th><th></th></tr></thead>
                <tbody>
                    <For each=move || groups.get() key=|g| (g.id, g.name.clone()) let:group>
                        {
                            let group_clone = group.clone();
                            let group_id = group.id;
                            let group_name = group.name.clone();
                            view! {
                                <tr>
                                    <td>{group.name.clone()}</td>
                                    <td class="data-table-actions">
                                        <button class="btn-small" on:click=move |_| start_edit(group_clone.clone())
                                            disabled=move || editing_group.get().is_some() || creating_group.get()
                                        >{move || i18n.get().t("general.edit")}</button>
                                        <button class="btn-small btn-danger" on:click=move |_| confirm_delete(group_id, group_name.clone())
                                            disabled=move || editing_group.get().is_some() || creating_group.get()
                                        >{move || i18n.get().t("general.delete")}</button>
                                    </td>
                                </tr>
                            }
                        }
                    </For>
                </tbody>
            </table>
        </div>
        </Show>
    }
}
