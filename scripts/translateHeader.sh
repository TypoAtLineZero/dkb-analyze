# Short script to translate csv header from
#     German to English

#!/usr/bin/env bash

HEADER="posting date,validation,status,payer,recipient,purpose,type,IBAN,amount,obligee,mandate,customer"

sed -i '1s/*./posting date,validation,status,payer,recipient,purpose,type,IBAN,amount,obligee,mandate,customer/' "$1"