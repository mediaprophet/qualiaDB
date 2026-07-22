use crate::cli::QpuAction;
use crate::qpu;

pub fn handle(action: &QpuAction, enable_qpu: bool) {
    if !enable_qpu {
        eprintln!("QPU commands require the --enable-qpu flag:");
        eprintln!("  qualia-cli --enable-qpu qpu <subcommand>");
        eprintln!();
        eprintln!("Subcommands: list-providers | configure | show | clear | test-connection | submit");
        std::process::exit(1);
    }
    let data_dir = std::env::var("QUALIA_DATA_DIR").unwrap_or_else(|_| ".".to_string());
    match action {
        QpuAction::ListProviders => {
            qpu::run_list_providers();
        }
        QpuAction::Configure {
            provider,
            api_key,
            endpoint,
            hub,
            group,
            project,
            instance,
            subscription_id,
            resource_group,
            workspace,
            location,
            access_key_id,
            secret_access_key,
            region,
            s3_bucket,
            project_id,
            processor_id,
            service_account_key_path,
            user_id,
            qpu_id,
            backend,
            machine,
        } => {
            qpu::run_configure(
                &data_dir,
                provider,
                api_key.as_deref(),
                endpoint.as_deref(),
                hub.as_deref(),
                group.as_deref(),
                project.as_deref(),
                instance.as_deref(),
                subscription_id.as_deref(),
                resource_group.as_deref(),
                workspace.as_deref(),
                location.as_deref(),
                access_key_id.as_deref(),
                secret_access_key.as_deref(),
                region.as_deref(),
                s3_bucket.as_deref(),
                project_id.as_deref(),
                processor_id.as_deref(),
                service_account_key_path.as_deref(),
                user_id.as_deref(),
                qpu_id.as_deref(),
                backend.as_deref(),
                machine.as_deref(),
            );
        }
        QpuAction::Show { provider } => {
            qpu::run_show(&data_dir, provider.as_deref());
        }
        QpuAction::Clear { provider } => {
            qpu::run_clear(&data_dir, provider);
        }
        QpuAction::TestConnection { provider } => {
            qpu::run_test_connection(&data_dir, provider);
        }
        QpuAction::Submit {
            provider,
            problem_type,
            qubits,
            shots,
        } => {
            qpu::run_submit(&data_dir, provider, problem_type, *qubits, *shots);
        }
    }
}
