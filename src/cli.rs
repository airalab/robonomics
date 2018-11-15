use futures::{future, Future, sync::oneshot};
use std::cell::RefCell;
use tokio::runtime::Runtime;
pub use substrate_cli::{VersionInfo, IntoExit, error};
use substrate_cli::{prepare_execution, Action, informant};
use service::Factory;
use substrate_service::{Components, ServiceFactory, Roles as ServiceRoles};
use chain_spec;
use std::ops::Deref;

/// Parse command line arguments into service configuration.
pub fn run<I, T, E>(args: I, exit: E, version: VersionInfo) -> error::Result<()> where
	I: IntoIterator<Item = T>,
	T: Into<std::ffi::OsString> + Clone,
	E: IntoExit,
{
	let description = version.description.clone();
	let executable_name = version.executable_name.clone();
	let author = version.author.clone();

	match prepare_execution::<Factory, _, _, _, _>(args, exit, version, load_spec, &executable_name)? {
		Action::ExecutedInternally => (),
		Action::RunService((config, exit)) => {
			info!("{} ({})", description, executable_name);
			info!("  version {}", config.full_version());
			info!("  by {}, 2018", author);
			info!("  powered by Substrate");
			info!("Chain specification: {}", config.chain_spec.name());
			info!("Node name: {}", config.name);
			info!("Roles: {:?}", config.roles);
			let mut runtime = Runtime::new()?;
			let executor = runtime.executor();
			match config.roles == ServiceRoles::LIGHT {
				true => run_until_exit(&mut runtime, Factory::new_light(config, executor)?, exit)?,
				false => run_until_exit(&mut runtime, Factory::new_full(config, executor)?, exit)?,
			}
		}
	}
	Ok(())
}

fn load_spec(id: &str) -> Result<Option<chain_spec::ChainSpec>, String> {
	Ok(match chain_spec::Alternative::from(id) {
		Some(spec) => Some(spec.load()?),
		None => None,
	})
}

fn run_until_exit<T, C, E>(
	runtime: &mut Runtime,
	service: T,
	e: E,
) -> error::Result<()>
	where
		T: Deref<Target=substrate_service::Service<C>>,
		C: Components,
		E: IntoExit,
{
	let (exit_send, exit) = exit_future::signal();

	let executor = runtime.executor();
	informant::start(&service, exit.clone(), executor.clone());

	let _ = runtime.block_on(e.into_exit());
	exit_send.fire();
	Ok(())
}

// handles ctrl-c
pub struct Exit;
impl IntoExit for Exit {
	type Exit = future::MapErr<oneshot::Receiver<()>, fn(oneshot::Canceled) -> ()>;
	fn into_exit(self) -> Self::Exit {
		// can't use signal directly here because CtrlC takes only `Fn`.
		let (exit_send, exit) = oneshot::channel();

		let exit_send_cell = RefCell::new(Some(exit_send));
		ctrlc::set_handler(move || {
			if let Some(exit_send) = exit_send_cell.try_borrow_mut().expect("signal handler not reentrant; qed").take() {
				exit_send.send(()).expect("Error sending exit notification");
			}
		}).expect("Error setting Ctrl-C handler");

		exit.map_err(drop)
	}
}
