<img width="577" height="100" alt="image" src="https://github.com/user-attachments/assets/25d09ab8-ef22-4f94-a378-31489c22905b" />

<hr>

RustPOS is a simple web-based Point of Sale system written in Rust. It is very easy to get going, does not require any additional setup and consists of a single executable for everything, with an additional optional print client available should you want to run the main program on a separate dedicated machine.

This is a simple implementation using modern technologies. It was born out of my frustration with free and open source POS software often being very brittle and relying on heavyweight technology stacks.

> [!NOTE]
> This project is a functional proof of concept and not certified to be used in a production environment.
> It also may not meet the requirements of your fiscal authorities. Please check your local laws and regulations before using this software to perform sales.

<img width="994" height="915" alt="image" src="https://github.com/user-attachments/assets/4f5e2971-cb09-482e-a3dc-591ee162f7f0" />


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

## Installation

RustPOS is provided in various installable packages. Simply install the applicable one. System services are provided for starting/stopping the program: ```sudo systemctl start rustpos``` starts the main program. The optional remote printer client can be started with ```sudo systemctl start rustpos-printclient```

To start the programs on system boot, enable the services as follows:

```sudo systemctl enable rustpos```
or
```sudo systemctl enable rustpos-printclient```

## Initial Setup

After installing and starting RustPOS, open the url https://127.0.0.1:3000/ in your browser. A language selection screen should be visible. Note that the language and all settings can be changed during operation too.

<img width="674" height="505" alt="image" src="https://github.com/user-attachments/assets/80f2ee14-8292-4653-acfe-f55e86f7fc61" />

Then select the desired currency:

<img width="654" height="454" alt="image" src="https://github.com/user-attachments/assets/200f0b0c-deea-4ec8-80d5-9b6fa996e5eb" />

Afterwards, an admin account with a random PIN will be created. *Remember that PIN*, it will not be shown afterwards! If a receipt printer is connected, the PIN will also be printed out.

<img width="653" height="517" alt="image" src="https://github.com/user-attachments/assets/051dd83b-535b-40a1-8d3c-c82304ef5da8" />

Continue and log in with the admin account:

<img width="494" height="328" alt="image" src="https://github.com/user-attachments/assets/c2b6ae62-8768-47e8-be4a-f06ee171ec68" />

<img width="492" height="646" alt="image" src="https://github.com/user-attachments/assets/c09fbc33-ff5d-4908-942a-6e7b62dbf8c0" />

You should be greeted by the main POS screen, which will be empty because there are no categories and items defined:

<img width="992" height="340" alt="image" src="https://github.com/user-attachments/assets/17704acb-c913-4a5e-9528-0ae2541ca5e9" />

Start by defining some categories:

<img width="989" height="492" alt="image" src="https://github.com/user-attachments/assets/2d7f5ea6-132b-4fdb-90ac-c746e088eac1" />

Then add the actual items for sale. You can upload item images and set a stock amount. If you check "Kitchen item", the item will be sent to and tracked by the kitchen display if ordered.

<img width="990" height="534" alt="image" src="https://github.com/user-attachments/assets/18cc161a-5f4e-433b-99f8-b0e207a63ed0" />

Next, we recommend setting up user accounts by clicking on "Settings":

<img width="992" height="366" alt="image" src="https://github.com/user-attachments/assets/e6f92477-366b-4cf2-bb2e-3628f8795589" />

You can create admins, cashiers and cooks, and set their PIN. Cashiers can only register sales and monitor the kitchen progress, cooks can only access and manage the kitchen display.

> [!NOTE]
> Now may be a good time to change the admin PIN!

<img width="991" height="644" alt="image" src="https://github.com/user-attachments/assets/ae9e4d51-557e-46f3-9adb-4042d7eddca4" />

## Basic operation

Making a sale is simple: Create a new transaction, optionally entering a customer name first. If you enter no name, then the customer will be labeled as "Walk-in". Customer names are useful for running tabs.

<img width="965" height="348" alt="image" src="https://github.com/user-attachments/assets/33b66c23-ed7e-4a09-bb20-ba8167e6dc2d" />

Now add items to the sale:

<img width="955" height="823" alt="image" src="https://github.com/user-attachments/assets/4f0dae0d-a91e-42de-a5d8-2ad0b0921614" />

You can press any of the quick cash buttons to display the change amount. Pressing *Checkout* will close the sale and print the receipt. Pressing *Back* will keep the transaction open but allow a different transaction to start. This is the equivalent to a running tab, and you can use the customer name feature to quickly find the correct transaction in the open transaction list:

<img width="968" height="360" alt="image" src="https://github.com/user-attachments/assets/4dd8635f-c65b-4d33-88ac-ae269be1ab9c" />

Once a sale is closed, the last sale's change value will still be displayed so you can fetch change from the drawer:

<img width="958" height="273" alt="image" src="https://github.com/user-attachments/assets/f430806b-cae7-4384-901b-5cbe1b8dca24" />

You can also always go to the transactions list and expand any transaction to see the money paid and the calculated change:

<img width="986" height="401" alt="image" src="https://github.com/user-attachments/assets/5b6db943-ac9e-4cc6-ba43-9f94c2e6cff4" />

## Receipt Printer Support

RustPOS will enumerate all receipt printers connected via serial port or USB, and use the first one it finds. Obviously this won't work for all setups, but for this proof of concept it should suffice.
The printout is designed for 80mm receipt printers and have been tested using a Munbyn ITPP098 connected via USB.

<img width="713" height="867" alt="image" src="https://github.com/user-attachments/assets/9334a0c5-aefa-4bfa-b5c0-d5df42c33415" />

## Remote Printer Support

It may be necessary to run RustPOS on a dedicated machine or even in the cloud. In these cases, connecting a printer to the server may not be possible, and the printing should happen at the point of sale. For this reason there is a seperate print client available. This print client will connect to the main RustPOS installation via websocket, authenticate, download the receipt logo from the server, and then wait for print jobs.

> [!NOTE]
> You do not need to install the print client if you run RustPOS itself on your POS. It will detect locally connected printers and use them directly.

The print client will install in ```/opt/rustpos-printclient``` and the configuration file is ```/opt/rustpos-printclient/printclient.toml```

First, on the main RustPOS administration page, configure a *Printer Passphrase*:

<img width="454" height="230" alt="image" src="https://github.com/user-attachments/assets/188efb07-47bf-4a28-a94e-ca289879f9b0" />

Then edit ```/opt/rustpos-printclient/printclient.toml``` on the machine the printer is connected to, and insert the passphrase as well as the address of the main RustPOS installation.

> [!NOTE]
> If your main RustPOS installation is available via HTTPS, the ```server_url``` should start with "wss", like in this example:
>
>```server_url = "wss://myserver.example.com"```

The receipt logo image will be transfered from the main program after authentication. However, it can also be overridden in the print client configuration TOML.

## Kitchen display

Cooks can access the kitchen display via the url ```/kitchen```. They are automatically redirected to that URL when logging on.

<img width="1365" height="642" alt="image" src="https://github.com/user-attachments/assets/8992a079-c2d0-4de8-ad9e-ddda83e71953" />

On the POS, there's a Kitchen tab that lets the cashier check the live status of every kitchen order.

<img width="1665" height="409" alt="image" src="https://github.com/user-attachments/assets/0f2d4e67-b0fc-4ebc-962b-e8069eab526c" />

##  Customer display

A customer display can be accessed at the url ```/display```. This will show items in the order as well as the total. The transaction will remain visible for one minute after the sale has been closed.

<img width="1215" height="710" alt="image" src="https://github.com/user-attachments/assets/f577bf40-be74-4c87-b5d7-82f8ec2a573a" />

## Reports

RustPOS supports sales reports with CSV export.

<img width="1339" height="1004" alt="image" src="https://github.com/user-attachments/assets/9035bc5b-f9ee-425c-a2e3-58996f3bd198" />

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
