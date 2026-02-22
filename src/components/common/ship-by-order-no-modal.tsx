import { useState } from "react";
import {
  NumberInput,
  DateInput,
  Button,
  Modal,
  ModalContent,
  ModalHeader,
  ModalBody,
  ModalFooter,
  DateValue,
} from "@heroui/react";

import {
    CalendarDate,
    parseDate,
} from '@internationalized/date';

interface SettingsModalProps {
  isOpen: boolean;
  onClose: (orderNo: number | null, shipQuantity: number | null, shipDate: CalendarDate | null) => void;
}

function ShipByOrderNoModal({ isOpen, onClose }: SettingsModalProps) {
  const [orderNo, setOrderNo] = useState<number>(0);
  const [shipQuantity, setShipQuantity] = useState<number>(0);
  const [shipDate, setShipDate] = useState<CalendarDate | null>(null);

  const renderDialogBody = () => (
    <div style={{ padding: "16px 0" }}>
      <div style={{ marginBottom: "24px" }}>
        <h3
          style={{
            fontSize: "16px",
            fontWeight: 600,
            marginBottom: "16px",
            color: "#333",
          }}
        >
          注文書NOで出庫
        </h3>

        <div style={{ marginBottom: "20px" }}>
          <label
            style={{
              display: "block",
              marginBottom: "8px",
              fontWeight: 500,
              color: "#333",
            }}
          >
            注文書NO
          </label>
          <NumberInput value={orderNo} onValueChange={setOrderNo} />
          <label
            style={{
              display: "block",
              marginBottom: "8px",
              fontWeight: 500,
              color: "#333",
            }}
          >
            数量
          </label>
          <NumberInput value={shipQuantity} onValueChange={setShipQuantity} />
          <label
            style={{
              display: "block",
              marginBottom: "8px",
              fontWeight: 500,
              color: "#333",
            }}
          >
            出荷日
          </label>
          <DateInput
            value={shipDate as DateValue | null}
            onChange={setShipDate as (value: DateValue | null) => void}
            errorMessage={(value) => {
              if (value.isInvalid) {
                return "日付を入力してください。";
              }
            }}
          />
        </div>
      </div>
    </div>
  );

  return (
    <Modal
      isOpen={isOpen}
      onClose={() => onClose(null, null, null)}
      size="4xl"
      scrollBehavior="inside"
      classNames={{
        base: "max-h-[80vh]",
        body: "py-6",
      }}
    >
      <ModalContent>
        {() => (
          <>
            <ModalHeader className="flex flex-col gap-1">
              <h2 className="text-xl font-semibold">
                出庫(ORDER)
              </h2>
            </ModalHeader>
            <ModalBody>
                <div className="flex-1 min-w-0">{renderDialogBody()}</div>
            </ModalBody>
            <ModalFooter>
              <Button color="primary" onPress={() => onClose(orderNo, shipQuantity, shipDate)}>
                出庫
              </Button>
            </ModalFooter>
          </>
        )}
      </ModalContent>
    </Modal>
  );
}

export default ShipByOrderNoModal;
