<img width="577" height="100" alt="image" src="https://github.com/user-attachments/assets/25d09ab8-ef22-4f94-a378-31489c22905b" />

<hr>

RustPOS is a simple web-based Point of Sale system written in Rust. It is very easy to get going, does not require any additional setup and consists of a single executable for everything, with an additional optional print client available should you want to run the main program on a separate dedicated machine.

This is a simple implementation using modern technologies. It was born out of my frustration with free and open source POS software often being very brittle and relying on heavyweight technology stacks.

> [!NOTE]
> This project is a functional proof of concept and not certified to be used in a production environment.
> It also may not meet the requirements of your fiscal authorities. Please check your local laws and regulations before using this software to perform sales.

<img width="1181" height="926" alt="image" src="https://github.com/user-attachments/assets/47ed4d5f-25cf-4da2-951e-4404e58e518b" />


## Features

* Completely configurable categories and items for sale via web UI
* Supports running tabs
* Change calculation
* Quick cash function
* Sales report generation: day, month, and custom date range reports with CSV export
* POS printer support (built into main application)
* Optional remote printer client (for dedicated server/cloud setups)
* Kitchen display
* User accounts and user roles (admin, cashier, cook)
* Simple inventory tracking
* Bright and Dark mode support
* All-Rust solution
* Leptos and Webassembly powered web UI, no javascript
* SQLite database
* Localized into several languages


<img width="1180" height="590" alt="image" src="https://github.com/user-attachments/assets/593e7a9a-544b-419e-be8a-df64b5fdfb8f" />

<img width="1179" height="1064" alt="image" src="https://github.com/user-attachments/assets/839da9a7-8090-42a3-b1cd-dec99cea7302" />

## Kitchen display

Cooks can access the kitchen display via the url /kitchen
Note that there is currently no security that prevents them from accessing the POS functions. Ideally this would be added along with user accounts and roles in the future.

<img width="1365" height="642" alt="image" src="https://github.com/user-attachments/assets/8992a079-c2d0-4de8-ad9e-ddda83e71953" />

On the POS, there's a Kitchen tab that lets the cashier check the live status of every kitchen order.

<img width="1665" height="409" alt="image" src="https://github.com/user-attachments/assets/0f2d4e67-b0fc-4ebc-962b-e8069eab526c" />

## Installation

RustPOS is provided in various installable packages. Simply install the applicable one. System services are provided for starting/stopping the program: ```sudo systemctl start rustpos``` starts the main program. The optional remote printer client can be started with ```sudo systemctl start rustpos-printclient```

To start the programs on system boot, enable the services as follows:

```sudo systemctl enable rustpos```
or
```sudo systemctl enable rustpos-printclient```

## Receipt Printer Support

RustPOS will enumerate all receipt printers connected via serial port or USB, and use the first one it finds. Obviously this won't work for all setups, but for this proof of concept it should suffice.
The printout is designed for 80mm receipt printers and have been tested using a Munbyn ITPP098 connected via USB.

<img width="713" height="867" alt="image" src="https://github.com/user-attachments/assets/9334a0c5-aefa-4bfa-b5c0-d5df42c33415" />

## Remote Printer Support

It may be necessary to run RustPOS on a dedicated machine or even in the cloud. In these cases, connecting a printer to the server may not be possible, and the printing should happen at the point of sale. For this reason there is a seperate print client available. This print client will connect to the main RustPOS installation via websocket, authenticate, download the receipt log from the server, and then wait for print jobs.

> [!NOTE]
> You do not need to install the print client if you run RustPOS itself on your POS. It will detect locally connected printers and use them directly.

The print client will install in ```/opt/rustpos-printclient``` and the configuration file is ```/opt/rustpos-printclient/printclient.toml```

First, on the main RustPOS administration page, configure a *Printer Passphrase*. Then edit ```/opt/rustpos-printclient/printclient.toml``` on the machine the printer is connected to, and insert the passphrase as well as the address of the main RustPOS installation.

> [!NOTE]
> If your main RustPOS installation is available via HTTPS, the ```server_url``` should start with "wss", like in this example:
>
>```server_url = "wss://myserver.example.com"```


## Customization

### Logo image

It is possible to change the logo used on the web UI. After installing the .deb package, the logo is at ```/opt/rustpos/site/logo_site.png```

### Receipt image

The receipt image is at ```/opt/rustpos/data/logo_receipt.png``` and can be replaced with your own B/W image.

## Data location

Item images and the entire POS database is stored in the ```data``` directory. If you installed via .deb package, the location will be ```/opt/rustpos/data```

## Manual Compilation

### Prerequisites
```
# Install Trunk (build tool for Rust WASM apps)
cargo install trunk

# Install WASM target
rustup target add wasm32-unknown-unknown
```

### Compiling

The build script ```build.sh``` will build the application and copy all the neccessary files into the "rustpos" subdirectory. 
You can copy that directory anywhere you want for a more permanent installation.

```
# Build application
./build.sh

# Run application
cd rustpos
./rustpos
```
